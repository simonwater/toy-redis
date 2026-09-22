use crate::Args;
use std::fmt::Write;

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
pub enum SectionType {
    Replication,
    All,
}

impl SectionType {
    pub fn new(name: Option<String>) -> Self {
        match name.map(|s| s.to_uppercase()).as_deref() {
            Some("REPLICATION") => Self::Replication,
            _ => Self::All,
        }
    }
}

pub struct Replication {
    role: String,
    //connected_slaves: u32,
    master_replid: String,
    master_repl_offset: i32,
}

impl Replication {
    fn new(args: &Args) -> Self {
        let role: String = if args.replicaof.is_some() {
            "slave".into()
        } else {
            "master".into()
        };
        Self {
            role,
            master_replid: "8371b4fb1155b71f4a04d3e1bc3e18c4a990aeeb".into(),
            master_repl_offset: 0,
        }
    }

    fn output(&self, out: &mut String) {
        let _ = writeln!(out, "# Replication");
        let _ = writeln!(out, "role:{}", self.role);
        //let _ = writeln!(out, "connected_slaves:{}", self.connected_slaves);
        let _ = writeln!(out, "master_replid:{}", self.master_replid);
        let _ = writeln!(out, "master_repl_offset:{}", self.master_repl_offset);
    }
}

pub struct Information {
    repl: Replication,
}

impl Information {
    pub fn new(args: &Args) -> Self {
        Self {
            repl: Replication::new(args),
        }
    }

    pub fn output(&self, section: SectionType, out: &mut String) {
        if section == SectionType::All || section == SectionType::Replication {
            self.repl.output(out);
        }
    }

    pub fn get_master_replid(&self) -> &str {
        &self.repl.master_replid
    }
}
