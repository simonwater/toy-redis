use crate::MemoryDB;
use crate::Value;
use crate::command::imme_command::ImmeCommand;
use anyhow::bail;
use anyhow::{Result, anyhow};
use bytes::Bytes;
use std::collections::HashMap;
use std::sync::Arc;
use std::vec::IntoIter;

pub enum TransCommand {
    Watch(IntoIter<Value>),
    Multi,
    Exec,
    Discard,
}

impl TransCommand {
    pub fn execute(self, db: &Arc<MemoryDB>, trans: &mut Transaction) -> Result<Value> {
        match self {
            Self::Multi => trans.start(),
            Self::Exec => trans.exec(db),
            Self::Discard => trans.discard(),
            Self::Watch(args) => trans.watch(args, db),
        }
    }
}

pub struct Transaction {
    commands: Option<Vec<ImmeCommand>>,
    watchs: HashMap<Bytes, u64>,
}

impl Transaction {
    pub fn new() -> Self {
        Self {
            commands: None,
            watchs: HashMap::with_capacity(16),
        }
    }

    pub fn is_started(&self) -> bool {
        self.commands.is_some()
    }

    pub(super) fn add_command(&mut self, cmd: ImmeCommand) -> Result<Value> {
        let commands = self
            .commands
            .as_mut()
            .ok_or_else(|| anyhow!("ERR Transaction is not started, can not add command."))?;
        commands.push(cmd);
        Ok(Value::SimpleStrings("QUEUED".into()))
    }

    pub(super) fn start(&mut self) -> Result<Value> {
        if self.commands.is_none() {
            self.commands = Some(Vec::with_capacity(16));
        }

        Ok(Value::SimpleStrings("OK".into()))
    }

    fn exec(&mut self, db: &Arc<MemoryDB>) -> Result<Value> {
        let commands = self
            .commands
            .take()
            .ok_or_else(|| anyhow!("ERR EXEC without MULTI"))?;

        let mut res: Vec<Value> = Vec::with_capacity(commands.len());
        for cmd in commands.into_iter() {
            match &cmd {
                ImmeCommand::Set(iter) | ImmeCommand::Get(iter) | ImmeCommand::Incr(iter) => {
                    let key = iter
                        .clone()
                        .next()
                        .ok_or_else(|| anyhow!("command missing key!"))?
                        .into_bulk_bytes()?;
                    if self.is_dirty(key, db) {
                        bail!("queued commands discarded")
                    }
                }
                _ => {}
            };
            let cmd_res = match cmd.execute(db, self) {
                Ok(val) => val,
                Err(e) => Value::SimpleErrors(format!("{}", e)),
            };
            res.push(cmd_res);
        }

        Ok(Value::Arrays(res))
    }

    fn discard(&mut self) -> Result<Value> {
        self.commands
            .take()
            .ok_or_else(|| anyhow!("ERR DISCARD without MULTI"))?;

        Ok(Value::SimpleStrings("OK".into()))
    }

    fn watch(&mut self, arg_iter: IntoIter<Value>, db: &Arc<MemoryDB>) -> Result<Value> {
        if self.is_started() {
            bail!("ERR WATCH inside MULTI is not allowed")
        }
        for key in arg_iter {
            let key = key.into_bulk_bytes()?;
            let version = db.get_version(&key).unwrap_or(0);
            self.watchs.insert(key, version);
        }
        Ok(Value::SimpleStrings("OK".into()))
    }

    fn is_dirty(&self, key: Bytes, db: &Arc<MemoryDB>) -> bool {
        let Some(watch_ver) = self.watchs.get(&key).copied() else {
            return false;
        };
        let db_ver = db.get_version(&key).unwrap_or(0);
        return watch_ver != db_ver;
    }
}
