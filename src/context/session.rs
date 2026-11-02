use anyhow::{Result, anyhow};

use crate::{ConnectionHandler, Subscription, Transaction};

/// 客户端状态
pub struct Session {
    trans: Option<Transaction>,
    conn: Option<ConnectionHandler>,
    subscribe: Option<Subscription>,
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

    pub fn with_subscription(mut self) -> Self {
        self.subscribe = Some(Subscription::new());
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

    pub fn subscription_ref(&self) -> Result<&Subscription> {
        self.subscribe
            .as_ref()
            .ok_or_else(|| anyhow!("subscription is not init in current session."))
    }

    pub fn subscription_mut(&mut self) -> Result<&mut Subscription> {
        self.subscribe
            .as_mut()
            .ok_or_else(|| anyhow!("subscription is not init in current session."))
    }

    pub fn is_subscribed_mode(&self) -> bool {
        self.subscribe
            .as_ref()
            .map(|s| !s.is_empty())
            .unwrap_or(false)
    }
}
