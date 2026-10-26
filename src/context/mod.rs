mod session;

pub use session::Session;

use crate::{Information, MemoryDB, ReplHub};
use anyhow::{Result, anyhow};
use clap::Parser;

#[derive(Parser, Debug, Clone)]
pub struct RedisArgs {
    #[arg(short, long, default_value = "6379")]
    pub port: String,
    #[arg(long)]
    pub replicaof: Option<String>,
    #[arg(long, default_value = ".")]
    pub dir: String,
    #[arg(long)]
    pub dbfilename: Option<String>,
    #[arg(long, default_value = "no")]
    pub appendonly: String,
    #[arg(long, default_value = "appendonlydir")]
    pub appenddirname: String,
    #[arg(long, default_value = "appendonly.aof")]
    pub appendfilename: String,
    #[arg(long, default_value = "everysec")]
    pub appendfsync: String,
}

impl RedisArgs {
    pub fn get_arg(&self, param_name: &str) -> Option<&str> {
        match param_name.to_lowercase().as_str() {
            "port" => Some(self.port.as_str()),
            "replicaof" => self.replicaof.as_deref(),
            "dir" => Some(self.dir.as_str()),
            "dbfilename" => self.dbfilename.as_deref(),
            "appendonly" => Some(self.appendonly.as_str()),
            "appenddirname" => Some(self.appenddirname.as_str()),
            "appendfilename" => Some(self.appendfilename.as_str()),
            "appendfsync" => Some(self.appendfsync.as_str()),
            _ => None,
        }
    }
}

pub struct Context {
    db: MemoryDB,
    args: RedisArgs,
    info: Information,
    repl_hub: ReplHub,
}

impl Context {
    pub fn new() -> Result<Self> {
        let mut args = RedisArgs::parse();
        if args.dir.as_str() == "." {
            args.dir = std::env::current_dir()?
                .to_str()
                .ok_or_else(|| anyhow!("current directory path contains non-UTF-8 characters"))?
                .to_string();
        }

        let info = Information::new(&args);
        let repl_hub = ReplHub::new();
        let db = match MemoryDB::from_context_args(&args) {
            Ok(db) => db,
            Err(e) => {
                eprintln!("{}", e);
                MemoryDB::new()
            }
        };

        Ok(Self {
            db,
            args,
            info,
            repl_hub,
        })
    }

    pub fn db_ref(&self) -> &MemoryDB {
        &self.db
    }

    pub fn args_ref(&self) -> &RedisArgs {
        &self.args
    }

    pub fn info_ref(&self) -> &Information {
        &self.info
    }

    pub fn repl_hub(&self) -> &ReplHub {
        &self.repl_hub
    }

    pub fn is_master(&self) -> bool {
        self.args_ref().replicaof.is_none()
    }
}
