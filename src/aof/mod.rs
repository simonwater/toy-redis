use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::PathBuf,
    thread,
};

use crate::{CmdExecutor, Command, Context, RedisArgs, Value};
use anyhow::{Result, bail};
use bytes::{Bytes, BytesMut};
use std::sync::Arc;
use std::sync::mpsc::{self, Sender};

struct Manifest {
    aof_filename: String,
}

pub struct AofHandler {
    _dir: PathBuf,
    cmd_sender: Option<Sender<Bytes>>,
    aof_file_path: Option<PathBuf>,
}

impl AofHandler {
    pub fn new(args: &RedisArgs) -> Result<Self> {
        let aof_dir_path = PathBuf::from(&args.dir).join(&args.appenddirname);
        let mut cmd_sender: Option<Sender<Bytes>> = None;
        let mut aof_file_path: Option<PathBuf> = None;

        if args.appendonly.as_str() == "yes" {
            // directory
            if !aof_dir_path.exists() {
                fs::create_dir_all(&aof_dir_path)?;
            }
            // manifest
            let manifest = Self::init_manifest(&aof_dir_path, &args.appendfilename)?;
            // aof file
            let (mut aof_file, file_path) =
                Self::init_aof_file(&aof_dir_path, &manifest.aof_filename)?;
            aof_file_path = Some(file_path);

            // aof working thread
            let (sender, receiver) = mpsc::channel();
            cmd_sender = Some(sender);
            thread::spawn(move || {
                loop {
                    match receiver.recv() {
                        Ok(bytes) => {
                            if let Err(e) = Self::write_bytes(bytes, &mut aof_file) {
                                eprintln!("aof thread write command bytes error: {}", e);
                            }
                        }
                        Err(e) => {
                            eprintln!("aof thread channel error: {}", e);
                            break;
                        }
                    }
                }
            });
        }

        let handler = Self {
            _dir: aof_dir_path,
            cmd_sender,
            aof_file_path,
        };
        Ok(handler)
    }

    fn write_bytes(bytes: Bytes, aof_file: &mut File) -> Result<()> {
        aof_file.write_all(&bytes.slice(..))?;
        aof_file.flush()?;
        Ok(())
    }

    fn init_manifest(dir: &PathBuf, appendfilename: &str) -> Result<Manifest> {
        let manifest_path = dir.join(format!("{}.manifest", appendfilename));
        let default_aof_filename = format!("{}.1.incr.aof", appendfilename);
        let mut file = OpenOptions::new()
            .read(true)
            .create(true) // 不存在则创建
            .append(true) // 开启追加模式
            .open(&manifest_path)?;
        let manifest = if file.metadata()?.len() == 0 {
            writeln!(file, "file {} seq 1 type i", default_aof_filename)?;
            Manifest {
                aof_filename: default_aof_filename.to_string(),
            }
        } else {
            let mut manifest_content = String::with_capacity(16);
            file.read_to_string(&mut manifest_content)?;
            let aof_filename = Self::extract_aof_filename(&manifest_content)
                .unwrap_or_else(|| &default_aof_filename);
            Manifest {
                aof_filename: aof_filename.to_string(),
            }
        };

        Ok(manifest)
    }

    fn init_aof_file(dir: &PathBuf, filename: &str) -> Result<(File, PathBuf)> {
        let file_path = dir.join(filename);
        let aof_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&file_path)?;
        Ok((aof_file, file_path))
    }

    fn extract_aof_filename(s: &str) -> Option<&str> {
        let mut parts = s.split_whitespace();
        if parts.next()? != "file" {
            return None;
        }
        parts.next()
    }

    pub fn append_command(&self, cmd: &Command) -> Result<()> {
        if let Some(sender) = self.cmd_sender.as_ref()
            && cmd.is_write()
        {
            sender.send(cmd.bytes.clone())?;
        }
        Ok(())
    }

    // 重放aof日志
    pub fn replay(&self, ctx: &Arc<Context>) -> Result<()> {
        if let Some(file_path) = &self.aof_file_path {
            let mut aof_file = OpenOptions::new().read(true).open(file_path)?;
            let mut buffer = BytesMut::with_capacity(4096);
            let mut tmp_buf = [0u8; 1024];
            loop {
                let read_cnt = aof_file.read(&mut tmp_buf)?;
                if read_cnt > 0 {
                    buffer.extend_from_slice(&tmp_buf[..read_cnt]);
                }
                if buffer.is_empty() {
                    return Ok(());
                }

                let (value, bytes) = Value::from(&mut buffer)?;
                match value {
                    Value::Arrays(values) => {
                        let command = Command::new(values, bytes)?;
                        // 直接执行，无需处理响应
                        let _response = CmdExecutor::execute(&command, ctx)?;
                    }
                    _ => bail!("command value must be resp array."),
                }
            }
        }

        Ok(())
    }
}
