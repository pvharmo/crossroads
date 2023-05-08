use std::sync::Arc;

use oauth2::{basic::BasicTokenType, StandardTokenResponse, EmptyExtraTokenFields};
use tokio::sync::Mutex;

pub type OneDriveToken = StandardTokenResponse<EmptyExtraTokenFields, BasicTokenType>;

#[derive(Clone)]
pub struct TokenStorage {
    token: Arc<Mutex<Option<OneDriveToken>>>
}

impl TokenStorage {
    pub async fn get(&self) -> Option<OneDriveToken> {
        (*self.token.lock().await).clone()
    }

    pub async fn set(&self, token: Option<OneDriveToken>) {
        let mut x = self.token.lock().await;
        *x = token;
    }

    pub fn new(token: Option<OneDriveToken>) -> Self {
        TokenStorage { token: Arc::new(Mutex::new(token)) }
    }
}