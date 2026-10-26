use std::{fs, path::PathBuf};

use crate::RedisArgs;
use anyhow::Result;

pub fn init_aof_env(args: &RedisArgs) -> Result<()> {
    if args.appendonly.as_str() == "yes" {
        let mut aop_dir_path = PathBuf::from(&args.dir);
        aop_dir_path.push(&args.appenddirname);
        if !aop_dir_path.exists() {
            fs::create_dir_all(aop_dir_path)?;
        }
    }
    Ok(())
}
