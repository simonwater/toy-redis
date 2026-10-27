use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::PathBuf,
    sync::RwLock,
};

use crate::{Command, RedisArgs};
use anyhow::Result;

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
            // aof file
            let aof_file_name = format!("{}.1.incr.aof", &args.appendfilename);
            let aof_file_path = aof_dir_path.join(&aof_file_name);
            let file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(aof_file_path)?;
            aof_file = Some(RwLock::new(file));

            // manifest file
            let manifest_path = aof_dir_path.join(format!("{}.manifest", &args.appendfilename));
            let mut manifest = OpenOptions::new()
                .create(true) // 不存在则创建
                .append(true) // 开启追加模式
                .open(manifest_path)?;
            writeln!(manifest, "file {} seq 1 type i", &aof_file_name)?;
            manifest_file = Some(manifest);
        }

        let handler = Self {
            _dir: aof_dir_path,
            aof_file,
            _manifest_file: manifest_file,
        };
        Ok(handler)
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
}
