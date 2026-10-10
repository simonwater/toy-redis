use crate::{Information, MemoryDB, ReplHub};
use clap::Parser;

#[derive(Parser, Debug, Clone)]
pub struct Args {
    #[arg(short, long, default_value = "6379")]
    pub port: String,
    #[arg(long)]
    pub replicaof: Option<String>,
}

pub struct Context {
    db: MemoryDB,
    args: Args,
    info: Information,
    repl_hub: ReplHub,
}

impl Context {
    pub fn new() -> Self {
        let args = Args::parse();
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

    pub fn args_ref(&self) -> &Args {
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
