use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::PathBuf,
};

use crate::RedisArgs;
use anyhow::Result;

pub fn init_aof_env(args: &RedisArgs) -> Result<()> {
    if args.appendonly.as_str() == "yes" {
        let mut aof_dir_path = PathBuf::from(&args.dir);
        aof_dir_path.push(&args.appenddirname);
        // directory
        if !aof_dir_path.exists() {
            fs::create_dir_all(&aof_dir_path)?;
        }
        // aof file
        let aof_file_name = format!("{}.1.incr.aof", &args.appendfilename);
        let aof_file_path = aof_dir_path.join(&aof_file_name);
        File::create(aof_file_path)?;

        // manifest file
        let manifest_path = aof_dir_path.join(format!("{}.manifest", &args.appendfilename));
        let mut manifest_file = OpenOptions::new()
            .create(true) // 不存在则创建
            .append(true) // 开启追加模式
            .open(manifest_path)?;
        writeln!(manifest_file, "file {} seq 1 type i", &aof_file_name)?;
    }
    Ok(())
}
