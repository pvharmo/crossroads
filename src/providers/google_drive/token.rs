extern crate google_drive3 as drive3;
use async_trait::async_trait;
use drive3::oauth2::storage::TokenStorage;

use std::{sync::{Arc, Mutex}, collections::HashMap};

use super::Token;

pub type MtTokenMap = Arc<Mutex<HashMap<String, Token>>>;

pub struct TokenStorageStrategy {
    pub token: MtTokenMap,
}

#[async_trait]
impl TokenStorage for TokenStorageStrategy {
    async fn set(&self, scopes: &[&str], token: Token) -> anyhow::Result<()> {
        let mut token_map = Arc::as_ref(&self.token).lock().unwrap();
        token_map.insert(scopes.join(" _ "), token.clone());
        Ok(())
    }

    async fn get(&self, target_scopes: &[&str]) -> Option<Token> {
        let token_map_mutex = &*Arc::as_ref(&self.token);
        let token_map = token_map_mutex.lock().unwrap();
        let token_result = token_map.get(target_scopes.join(" _ ").as_str()).clone();

        if token_result.is_some() {
            return Some(token_result.unwrap().clone())
        }

        None
    }
}