use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Seek, Write},
    path::PathBuf,
    sync::RwLock,
};

use crate::{CmdExecutor, Command, Context, RedisArgs, Value};
use anyhow::{Result, bail};
use bytes::BytesMut;
use std::sync::Arc;

pub struct AofHandler {
    _dir: PathBuf,
    aof_file: Option<RwLock<File>>,
    _manifest_file: Option<File>,
}

impl AofHandler {
    pub fn new(args: &RedisArgs) -> Result<Self> {
        let aof_dir_path = PathBuf::from(&args.dir).join(&args.appenddirname);
        let mut aof_file = None;
        let mut manifest_file = None;

        if args.appendonly.as_str() == "yes" {
            // directory
            if !aof_dir_path.exists() {
                fs::create_dir_all(&aof_dir_path)?;
            }
            let default_aof_filename = format!("{}.1.incr.aof", &args.appendfilename);

            // manifest file
            let manifest_path = aof_dir_path.join(format!("{}.manifest", &args.appendfilename));
            let mut manifest = OpenOptions::new()
                .read(true)
                .create(true) // 不存在则创建
                .append(true) // 开启追加模式
                .open(manifest_path)?;
            if manifest.metadata()?.len() == 0 {
                writeln!(manifest, "file {} seq 1 type i", &default_aof_filename)?;
            }

            // aof file
            let mut manifest_content = String::with_capacity(16);
            manifest.read_to_string(&mut manifest_content)?;
            let aof_filename = Self::extract_aof_filename(&manifest_content)
                .unwrap_or_else(|| &default_aof_filename);
            let aof_file_path = aof_dir_path.join(aof_filename);
            let file = OpenOptions::new()
                .read(true)
                .create(true)
                .append(true)
                .open(aof_file_path)?;

            manifest_file = Some(manifest);
            aof_file = Some(RwLock::new(file));
        }

        let handler = Self {
            _dir: aof_dir_path,
            aof_file,
            _manifest_file: manifest_file,
        };
        Ok(handler)
    }

    fn extract_aof_filename(s: &str) -> Option<&str> {
        let mut parts = s.split_whitespace();
        if parts.next()? != "file" {
            return None;
        }
        parts.next()
    }

    pub fn append_command(&self, cmd: &Command) -> Result<()> {
        if let Some(file_guard) = self.aof_file.as_ref()
            && cmd.is_write()
        {
            let mut file = file_guard.write().unwrap();
            file.write_all(&cmd.bytes.slice(..))?;
            file.flush()?;
        }
        Ok(())
    }

    pub fn replay(&self, ctx: &Arc<Context>) -> Result<()> {
        if let Some(file_guard) = self.aof_file.as_ref() {
            let mut file = file_guard.write().unwrap();
            file.rewind()?;
            let mut buffer = BytesMut::with_capacity(4096);
            let mut tmp_buf = [0u8; 1024];
            loop {
                let read_cnt = file.read(&mut tmp_buf)?;
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
