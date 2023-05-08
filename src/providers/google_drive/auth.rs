extern crate google_drive3 as drive3;

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use drive3::oauth2::storage::TokenInfo;
use drive3::{DriveHub, oauth2, hyper, hyper_rustls};
use drive3::oauth2::authenticator_delegate::{DefaultInstalledFlowDelegate, InstalledFlowDelegate};

use super::Token;
use super::{GoogleDrive, token::TokenStorageStrategy};

async fn browser_user_url(url: &str, need_code: bool) -> Result<String, String> {
    open::that(url).expect("An error occurred when trying to open web browser");
    let def_delegate = DefaultInstalledFlowDelegate;
    def_delegate.present_user_url(url, need_code).await
}

#[derive(Copy, Clone)]
struct InstalledFlowBrowserDelegate;

impl InstalledFlowDelegate for InstalledFlowBrowserDelegate {
    fn present_user_url<'a>(
        &'a self,
        url: &'a str,
        need_code: bool,
    ) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send + 'a>> {
        Box::pin(browser_user_url(url, need_code))
    }
}

impl GoogleDrive {
    pub async fn new(client_secret: String, tokens: HashMap<String, TokenInfo>) -> Result<GoogleDrive, Box<dyn std::error::Error>> {
        let secret = oauth2::parse_application_secret(client_secret.as_str()).unwrap();

        let mt_tokens = Arc::new(Mutex::new(tokens));

        let storage = Box::new(TokenStorageStrategy { token: mt_tokens.clone() });
        
        let auth = oauth2::InstalledFlowAuthenticator::builder(
            secret,
            oauth2::InstalledFlowReturnMethod::HTTPRedirect
        ).with_storage(storage).flow_delegate(Box::new(InstalledFlowBrowserDelegate)).build().await.unwrap();

        let hub = DriveHub::new(
            hyper::Client::builder().build(
                hyper_rustls::HttpsConnectorBuilder::new().with_native_roots().https_or_http().enable_http1().enable_http2().build()),
                auth);

        Ok(GoogleDrive { hub, tokens: mt_tokens.clone() })
    }
    
    pub fn tokens_map(&self) -> HashMap<String, Token> {
        (*self.tokens.as_ref().lock().unwrap()).clone()
    }
}