use crate::Command;
use crate::MemoryDB;
use crate::Value;
use anyhow::{Result, anyhow};
use std::sync::Arc;

pub struct Transaction {
    commands: Option<Vec<Command>>,
}

impl Transaction {
    pub fn new() -> Self {
        Self { commands: None }
    }

    pub fn handle_command(&mut self, cmd: Command, db: &Arc<MemoryDB>) -> Result<Value> {
        let res = match cmd {
            Command::Multi => self.start()?,
            Command::Exec => self.exec(db)?,
            Command::Discard => self.discard()?,
            cmd => {
                if let Some(commands) = self.commands.as_mut() {
                    commands.push(cmd);
                    Value::SimpleStrings("QUEUED".into())
                } else {
                    cmd.execute(db)?
                }
            }
        };

        Ok(res)
    }

    fn start(&mut self) -> Result<Value> {
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
            let cmd_res = match cmd.execute(db) {
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
}
