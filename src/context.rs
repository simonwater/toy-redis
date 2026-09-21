use crate::MemoryDB;
use clap::Parser;

#[derive(Parser, Debug, Clone)]
pub struct Args {
    #[arg(short, long, default_value = "6379")]
    pub port: String,
}

pub struct Context {
    db: MemoryDB,
    args: Args,
}

impl Context {
    pub fn new() -> Self {
        Self {
            db: MemoryDB::new(),
            args: Args::parse(),
        }
    }

    pub fn db_ref(&self) -> &MemoryDB {
        &self.db
    }

    pub fn args_ref(&self) -> &Args {
        &self.args
    }
}
