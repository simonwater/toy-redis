use crate::command::imme_command::ImmeCommand;
use crate::{CommandResponse, Context, Value};
use anyhow::bail;
use anyhow::{Result, anyhow};
use bytes::Bytes;
use std::collections::HashMap;
use std::sync::Arc;
use std::vec::IntoIter;

pub enum TransCommand {
    Watch(IntoIter<Value>),
    Unwatch,
    Multi,
    Exec,
    Discard,
}

impl TransCommand {
    pub fn execute(self, ctx: &Arc<Context>, trans: &mut Transaction) -> Result<CommandResponse> {
        match self {
            Self::Multi => trans.start(),
            Self::Exec => trans.exec(ctx),
            Self::Discard => trans.discard(),
            Self::Watch(args) => trans.watch(args, ctx),
            Self::Unwatch => trans.unwatch(),
        }
    }
}

pub struct Transaction {
    commands: Option<Vec<ImmeCommand>>,
    watchs: Option<HashMap<Bytes, u64>>,
}

impl Transaction {
    pub fn new() -> Self {
        Self {
            commands: None,
            watchs: None,
        }
    }

    pub fn is_started(&self) -> bool {
        self.commands.is_some()
    }

    pub(super) fn add_command(&mut self, cmd: ImmeCommand) -> Result<CommandResponse> {
        let commands = self
            .commands
            .as_mut()
            .ok_or_else(|| anyhow!("ERR Transaction is not started, can not add command."))?;
        commands.push(cmd);
        Ok(Value::SimpleStrings("QUEUED".into()).into())
    }

    pub(super) fn start(&mut self) -> Result<CommandResponse> {
        if self.commands.is_none() {
            self.commands = Some(Vec::with_capacity(16));
        }

        Ok(Value::SimpleStrings("OK".into()).into())
    }

    fn exec(&mut self, ctx: &Arc<Context>) -> Result<CommandResponse> {
        let commands = self
            .commands
            .take()
            .ok_or_else(|| anyhow!("ERR EXEC without MULTI"))?;
        let mut res: Vec<Value> = Vec::with_capacity(commands.len());

        if Self::check_dirty(ctx, self.watchs.take()) {
            return Ok(Value::NullArrays.into());
        }
        for cmd in commands.into_iter() {
            let cmd_res = match cmd.execute(ctx, self) {
                Ok(response) => match response {
                    CommandResponse::RespValue(value) => value,
                    _ => {
                        Value::SimpleErrors(format!("can not execute this command in transaction."))
                    }
                },
                Err(e) => Value::SimpleErrors(format!("{}", e)),
            };
            res.push(cmd_res);
        }

        Ok(Value::Arrays(res).into())
    }

    fn discard(&mut self) -> Result<CommandResponse> {
        self.commands
            .take()
            .ok_or_else(|| anyhow!("ERR DISCARD without MULTI"))?;

        self.watchs.take();
        Ok(Value::SimpleStrings("OK".into()).into())
    }

    fn watch(&mut self, arg_iter: IntoIter<Value>, ctx: &Arc<Context>) -> Result<CommandResponse> {
        if self.is_started() {
            bail!("ERR WATCH inside MULTI is not allowed")
        }

        let watchs = self
            .watchs
            .get_or_insert_with(|| HashMap::with_capacity(16));
        let db = ctx.db_ref();
        for key in arg_iter {
            let key = key.into_bulk_bytes()?;
            let version = db.get_version(&key).unwrap_or(0);
            watchs.insert(key, version);
        }
        Ok(Value::SimpleStrings("OK".into()).into())
    }

    fn unwatch(&mut self) -> Result<CommandResponse> {
        self.watchs.take();
        Ok(Value::SimpleStrings("OK".into()).into())
    }

    fn check_dirty(ctx: &Arc<Context>, watchs: Option<HashMap<Bytes, u64>>) -> bool {
        if let Some(watchs) = watchs.as_ref() {
            let db = ctx.db_ref();
            for (key, &ver) in watchs.iter() {
                let db_ver = db.get_version(&key).unwrap_or(0);
                if ver != db_ver {
                    return true;
                }
            }
        }
        false
    }
}
