use anyhow::{Result, anyhow};

use crate::{ConnectionHandler, Transaction, context::subscribe::Subscribe};

/// 客户端状态
pub struct Session {
    trans: Option<Transaction>,
    conn: Option<ConnectionHandler>,
    subscribe: Option<Subscribe>,
}

impl Session {
    pub fn new() -> Self {
        Self {
            trans: None,
            subscribe: None,
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

    pub fn with_subscribe(mut self) -> Self {
        self.subscribe = Some(Subscribe::new());
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

    pub fn subscribe_ref(&self) -> Result<&Subscribe> {
        self.subscribe
            .as_ref()
            .ok_or_else(|| anyhow!("subscribe is not init in current session."))
    }

    pub fn subscribe_mut(&mut self) -> Result<&mut Subscribe> {
        self.subscribe
            .as_mut()
            .ok_or_else(|| anyhow!("subscribe is not init in current session."))
    }
}
