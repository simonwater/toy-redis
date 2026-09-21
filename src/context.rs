use crate::{Information, MemoryDB};
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
}

impl Context {
    pub fn new() -> Self {
        let args = Args::parse();
        let info = Information::new(&args);
        Self {
            db: MemoryDB::new(),
            args,
            info,
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
}
