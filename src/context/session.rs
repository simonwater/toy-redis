use anyhow::{Result, anyhow};
use bytes::Bytes;
use std::collections::HashSet;

use crate::{ConnectionHandler, Transaction};

/// 客户端状态
pub struct Session {
    trans: Option<Transaction>,
    conn: Option<ConnectionHandler>,
    sub_chennals: HashSet<Bytes>,
}

impl Session {
    pub fn new() -> Self {
        Self {
            trans: None,
            sub_chennals: HashSet::with_capacity(8),
            conn: None,
        }
    }

    pub fn with_connection(mut self, conn: ConnectionHandler) -> Self {
        self.conn = Some(conn);
        self
    }

    pub fn with_transaction(mut self) -> Self {
        self.trans = Some(Transaction::new());
        self
    }

    pub fn trans_ref(&self) -> Result<&Transaction> {
        self.trans
            .as_ref()
            .ok_or_else(|| anyhow!("transaction is not init in current session."))
    }

    pub fn trans_mut(&mut self) -> Result<&mut Transaction> {
        self.trans
            .as_mut()
            .ok_or_else(|| anyhow!("transaction is not init in current session."))
    }

    pub fn connection_ref(&self) -> Result<&ConnectionHandler> {
        self.conn
            .as_ref()
            .ok_or_else(|| anyhow!("connection is not init in current session."))
    }

    pub fn connection_mut(&mut self) -> Result<&mut ConnectionHandler> {
        self.conn
            .as_mut()
            .ok_or_else(|| anyhow!("connection is not init in current session."))
    }

    pub fn add_chennal(&mut self, name: Bytes) {
        self.sub_chennals.insert(name);
    }

    pub fn chennal_count(&self) -> usize {
        self.sub_chennals.len()
    }
}
