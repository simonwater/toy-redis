use crate::{ConnectionHandler, Transaction};

pub struct Session {
    trans: Transaction,
    conn: ConnectionHandler,
}

impl Session {
    pub fn new(conn: ConnectionHandler) -> Self {
        Self {
            trans: Transaction::new(),
            conn,
        }
    }

    pub fn trans_ref(&self) -> &Transaction {
        &self.trans
    }

    pub fn trans_mut(&mut self) -> &mut Transaction {
        &mut self.trans
    }

    pub fn connection_ref(&self) -> &ConnectionHandler {
        &self.conn
    }

    pub fn connection_mut(&mut self) -> &mut ConnectionHandler {
        &mut self.conn
    }
}
