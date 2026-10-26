use std::{
    fs::{self, File},
    path::PathBuf,
};

use crate::RedisArgs;
use anyhow::Result;

pub fn init_aof_env(args: &RedisArgs) -> Result<()> {
    if args.appendonly.as_str() == "yes" {
        let mut aof_path = PathBuf::from(&args.dir);
        aof_path.push(&args.appenddirname);
        if !aof_path.exists() {
            fs::create_dir_all(&aof_path)?;
        }
        aof_path.push(format!("{}.1.incr.aof", &args.appendfilename));
        File::create(aof_path)?;
    }
    Ok(())
}
