mod session;

pub use session::Session;

use crate::{Information, MemoryDB, ReplHub};
use clap::Parser;

#[derive(Parser, Debug, Clone)]
pub struct RedisArgs {
    #[arg(short, long, default_value = "6379")]
    pub port: String,
    #[arg(long)]
    pub replicaof: Option<String>,
    #[arg(long)]
    pub dir: Option<String>,
    #[arg(long)]
    pub dbfilename: Option<String>,
}

impl RedisArgs {
    pub fn get_arg(&self, param_name: &str) -> Option<&str> {
        match param_name.to_lowercase().as_str() {
            "port" => Some(self.port.as_str()),
            "replicaof" => self.replicaof.as_deref(),
            "dir" => self.dir.as_deref(),
            "dbfilename" => self.dbfilename.as_deref(),
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
    pub fn new() -> Self {
        let args = RedisArgs::parse();
        let info = Information::new(&args);
        let repl_hub = ReplHub::new();
        Self {
            db: MemoryDB::new(),
            args,
            info,
            repl_hub,
        }
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
