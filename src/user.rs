use std::sync::{Arc, RwLock};

use crate::{Session, Value};
use bytes::Bytes;
use dashmap::DashMap;
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Copy)]
pub struct Flags {
    nopass: bool,
}

impl Flags {
    pub fn new() -> Self {
        Self { nopass: true }
    }

    pub fn set_nopass(&mut self, flag: bool) {
        self.nopass = flag;
    }

    pub fn into_value(self) -> Value {
        let mut arr: Vec<Value> = Vec::with_capacity(8);
        if self.nopass {
            arr.push(Bytes::from("nopass").into());
        }

        arr.into()
    }
}

#[derive(Debug, Clone)]
pub struct UserStore {
    flags: Flags,
    passwords: Vec<Bytes>,
}

impl UserStore {
    pub fn new() -> Self {
        Self {
            flags: Flags::new(),
            passwords: Vec::with_capacity(8),
        }
    }

    pub fn set_password(&mut self, password: &str) -> Bytes {
        self.flags.set_nopass(false);
        let pass_hash = Sha256::digest(password.as_bytes());
        let hex_string = format!("{:x}", pass_hash);
        let bytes: Bytes = Bytes::from(hex_string);
        self.passwords.push(bytes.clone());
        bytes
    }

    pub fn to_value(&self) -> Value {
        let mut arr: Vec<Value> = Vec::with_capacity(8);
        // flags
        arr.push(Bytes::from("flags").into());
        arr.push(self.flags.into_value());

        // passwords
        arr.push(Bytes::from("passwords").into());
        let pass_values: Value = self.passwords.clone().into();
        arr.push(pass_values.into());

        arr.into()
    }
}

pub struct User {
    store: RwLock<UserStore>,
}

impl User {
    pub fn new() -> Self {
        Self {
            store: RwLock::new(UserStore::new()),
        }
    }

    pub fn set_password(&self, password: &str) -> Bytes {
        self.store.write().unwrap().set_password(password)
    }

    pub fn to_value(&self) -> Value {
        self.store.read().unwrap().to_value()
    }
}

pub struct UsersHub {
    users: DashMap<Bytes, Arc<User>>,
}

impl UsersHub {
    pub fn new() -> Self {
        let default_user = Arc::new(User::new());
        let users = DashMap::with_capacity(16);
        users.insert(Bytes::from("default"), default_user);

        Self { users }
    }

    pub fn get_user(&self, name: &Bytes) -> Option<Arc<User>> {
        self.users.get(name).map(|e| e.value().clone())
    }

    pub fn current_user(&self, _session: &Session) -> Arc<User> {
        self.get_user(&Bytes::from("default")).unwrap()
    }
}
