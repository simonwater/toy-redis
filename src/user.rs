use crate::{Session, Value};
use anyhow::{Result, anyhow, bail};
use bytes::Bytes;
use dashmap::DashMap;
use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    sync::{Arc, RwLock},
};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UserToken {
    name: Bytes,
    pwd_hash: Bytes,
}

impl UserToken {
    pub fn new(name: Bytes, pwd_hash: Bytes) -> Self {
        Self { name, pwd_hash }
    }
}

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
    name: Bytes,
    flags: Flags,
    passwords: HashSet<Bytes>,
}

impl UserStore {
    pub fn new(name: Bytes) -> Self {
        Self {
            name,
            flags: Flags::new(),
            passwords: HashSet::with_capacity(8),
        }
    }

    fn validate_password(&self, password: &str) -> Result<()> {
        if password.is_empty() {
            bail!("password can not be empty!")
        }
        Ok(())
    }

    fn password_hash_bytes(&self, password: &str) -> Bytes {
        let pass_hash = Sha256::digest(password.as_bytes());
        let hex_string = format!("{:x}", pass_hash);
        Bytes::from(hex_string)
    }

    fn set_password(&mut self, password: &str) -> Result<Bytes> {
        self.validate_password(password)?;
        self.flags.set_nopass(false);
        let pwd_bytes = self.password_hash_bytes(password);
        self.passwords.insert(pwd_bytes.clone());
        Ok(pwd_bytes)
    }

    fn check_password(&self, password: &str) -> Option<Bytes> {
        let pwd_bytes = self.password_hash_bytes(password);
        if self.check_pwd_bytes(&pwd_bytes) {
            return Some(pwd_bytes);
        }
        None
    }

    fn check_pwd_bytes(&self, pwd_bytes: &Bytes) -> bool {
        self.passwords.contains(pwd_bytes)
    }

    fn to_value(&self) -> Value {
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
    pub fn new(name: Bytes) -> Self {
        Self {
            store: RwLock::new(UserStore::new(name)),
        }
    }

    pub fn set_password(&self, password: &str) -> Result<Bytes> {
        self.store.write().unwrap().set_password(password)
    }

    pub fn check_password(&self, password: &str) -> Option<Bytes> {
        self.store.read().unwrap().check_password(password)
    }

    pub fn is_nopass(&self) -> bool {
        self.store.read().unwrap().flags.nopass
    }

    pub fn name(&self) -> Bytes {
        self.store.read().unwrap().name.clone()
    }

    pub fn to_value(&self) -> Value {
        self.store.read().unwrap().to_value()
    }
}

pub struct UsersHub {
    users_by_name: DashMap<Bytes, Arc<User>>,
    users_by_token: DashMap<UserToken, Arc<User>>,
}

impl UsersHub {
    pub fn new() -> Self {
        let default_name = Bytes::from("default");
        let default_user = Arc::new(User::new(default_name.clone()));
        let users_by_name = DashMap::with_capacity(16);
        users_by_name.insert(default_name, default_user);

        Self {
            users_by_name,
            users_by_token: DashMap::with_capacity(16),
        }
    }

    pub fn get_user_by_name(&self, name: &Bytes) -> Option<Arc<User>> {
        self.users_by_name.get(name).map(|e| e.value().clone())
    }

    pub fn get_user_by_token(&self, token: &UserToken) -> Option<Arc<User>> {
        self.users_by_token.get(&token).map(|e| e.value().clone())
    }

    pub fn current_user(&self, session: &Session) -> Result<Arc<User>> {
        if let Some(user_token) = session.current_user_token() {
            match self.get_user_by_token(user_token) {
                Some(user) => return Ok(user),
                None => bail!("NOAUTH Authentication required."),
            }
        } else {
            // no auth, check default user
            let default_user = self.get_user_by_name(&Bytes::from("default")).unwrap();
            if default_user.is_nopass() {
                return Ok(default_user);
            } else {
                bail!("NOAUTH Authentication required.")
            }
        }
    }

    pub fn auth(&self, name: &Bytes, password: &str, session: &mut Session) -> Result<()> {
        let user = self
            .get_user_by_name(name)
            .ok_or_else(|| anyhow!("ERR user: {:?} is not exist.", name))?;
        if let Some(pwd_bytes) = user.check_password(password) {
            let token = UserToken::new(name.clone(), pwd_bytes);
            self.users_by_token.insert(token.clone(), user);
            session.set_user_token(token);
            Ok(())
        } else {
            bail!("WRONGPASS invalid username-password pair or user is disabled.")
        }
    }

    pub fn auth_user_token(&self, user: Arc<User>, token: UserToken, session: &mut Session) {
        self.users_by_token.insert(token.clone(), user);
        session.set_user_token(token);
    }
}
