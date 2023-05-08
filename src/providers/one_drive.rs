use async_trait::async_trait;
use onedrive_api::{OneDrive as OneDriveLib, FileName, DriveLocation, ItemLocation};

use oauth2::basic::{BasicClient, BasicTokenType, BasicErrorResponseType};
// Alternatively, this can be `oauth2::curl::http_client` or a custom client.
use oauth2::reqwest::http_client;
use oauth2::{
    AuthType, AuthUrl, AuthorizationCode, ClientId, CsrfToken, PkceCodeChallenge,
    RedirectUrl, Scope, TokenUrl, StandardTokenResponse, EmptyExtraTokenFields, PkceCodeVerifier, TokenResponse, StandardErrorResponse, Client, StandardTokenIntrospectionResponse, StandardRevocableToken, RevocationErrorResponseType,
};
use serde::{Serialize, Deserialize};
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use oauth2::url::Url;

use crate::interfaces::filesystem::{FileSystem, ObjectId};

type OneDriveToken = StandardTokenResponse<EmptyExtraTokenFields, BasicTokenType>;

#[derive(Serialize, Deserialize)]
pub struct OneDrive {
    token: Option<OneDriveToken>,
    client_id: String
}

fn listen_for_token(client: BasicClient, csrf_state: CsrfToken, pkce_code_verifier: PkceCodeVerifier) -> Result<StandardTokenResponse<EmptyExtraTokenFields, BasicTokenType>, ()> {
    // A very naive implementation of the redirect server.
    let listener = TcpListener::bind("127.0.0.1:3003").unwrap();
    for stream in listener.incoming() {
        if let Ok(mut stream) = stream {
            let code;
            let state;
            {
                let mut reader = BufReader::new(&stream);

                let mut request_line = String::new();
                reader.read_line(&mut request_line).unwrap();

                let redirect_url = request_line.split_whitespace().nth(1).unwrap();
                let url = Url::parse(&("http://localhost".to_string() + redirect_url)).unwrap();

                let code_pair = url
                    .query_pairs()
                    .find(|pair| {
                        let &(ref key, _) = pair;
                        key == "code"
                    })
                    .unwrap();

                let (_, value) = code_pair;
                code = AuthorizationCode::new(value.into_owned());

                let state_pair = url
                    .query_pairs()
                    .find(|pair| {
                        let &(ref key, _) = pair;
                        key == "state"
                    })
                    .unwrap();

                let (_, value) = state_pair;
                state = CsrfToken::new(value.into_owned());
            }

            let message = "Go back to your terminal :)";
            let response = format!(
                "HTTP/1.1 200 OK\r\ncontent-length: {}\r\n\r\n{}",
                message.len(),
                message
            );
            stream.write_all(response.as_bytes()).unwrap();

            println!("MS Graph returned the following code:\n{}\n", code.secret());
            println!(
                "MS Graph returned the following state:\n{} (expected `{}`)\n",
                state.secret(),
                csrf_state.secret()
            );

            // Exchange the code with a token.
            let token_result = client
                .exchange_code(code)
                // Send the PKCE code verifier in the token request
                .set_pkce_verifier(pkce_code_verifier)
                .request(http_client);

            if let Ok(token) = token_result {
                println!("MS Graph returned the following token:\n{:?}\n", token);
    
                // The server will terminate itself after collecting the first code.
                return Ok(token)
            } else {
                dbg!(token_result);
            }

            break;
        }
    }

    Err(())
}

impl OneDrive {
    fn new_client(client_id: String) -> Client<StandardErrorResponse<BasicErrorResponseType>, StandardTokenResponse<EmptyExtraTokenFields, BasicTokenType>, BasicTokenType, StandardTokenIntrospectionResponse<EmptyExtraTokenFields, BasicTokenType>, StandardRevocableToken, StandardErrorResponse<RevocationErrorResponseType>>
    {
        let graph_client_id = ClientId::new(client_id);
        let auth_url =
            AuthUrl::new("https://login.microsoftonline.com/common/oauth2/v2.0/authorize".to_string())
                .expect("Invalid authorization endpoint URL");

        let token_url =
            TokenUrl::new("https://login.microsoftonline.com/common/oauth2/v2.0/token".to_string())
                .expect("Invalid token endpoint URL");

        BasicClient::new(
            graph_client_id,
            None,
            auth_url,
            Some(token_url),
        ).set_auth_type(AuthType::RequestBody)
        // This example will be running its own server at localhost:3003.
        // See below for the server implementation.
        .set_redirect_uri(
            RedirectUrl::new("http://localhost:3003/redirect".to_string())
                .expect("Invalid redirect URL"),
        )
    }

    pub fn fetch_credentials(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let client = Self::new_client(self.client_id.clone());
    
        // Microsoft Graph supports Proof Key for Code Exchange (PKCE - https://oauth.net/2/pkce/).
        // Create a PKCE code verifier and SHA-256 encode it as a code challenge.
        let (pkce_code_challenge, pkce_code_verifier) = PkceCodeChallenge::new_random_sha256();
    
        // Generate the authorization URL to which we'll redirect the user.
        let (authorize_url, csrf_state) = client
            .authorize_url(CsrfToken::new_random)
            // This example requests read access to OneDrive.
            .add_scope(Scope::new(
                "https://graph.microsoft.com/Files.Read".to_string(),
            ))
            .set_pkce_challenge(pkce_code_challenge)
            .url();
    
        println!(
            "Open this URL in your browser:\n{}\n",
            authorize_url.to_string()
        );

        open::that(authorize_url.to_string()).expect("Could not open browser to authenticate to OneDrive.");

        let token = listen_for_token(client, csrf_state, pkce_code_verifier).expect("Error fetching OneDrive access token");

        self.token = Some(token);

        Ok(())
    }

    pub fn new (token: Option<OneDriveToken>, client_id: String) -> OneDrive {
        OneDrive { token, client_id }
    }

    pub fn refresh_token(&mut self, token: OneDriveToken, client_id: String) -> Result<(), Box<dyn std::error::Error>> {
        let client = Self::new_client(client_id);
        let token = client.exchange_refresh_token(token.refresh_token().unwrap()).request(http_client)?;
        self.token = Some(token);

        Ok(())
    }
}

#[async_trait]
impl FileSystem for OneDrive {
    async fn read_file(&self, object_id: crate::interfaces::filesystem::ObjectId) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        todo!()
    }

    async fn write_file(&self, object_id: crate::interfaces::filesystem::ObjectId, content: Vec<u8>) -> Result<(), Box<dyn std::error::Error>> {
        todo!()
    }

    async fn delete(&self, object_id: crate::interfaces::filesystem::ObjectId) -> Result<(), Box<dyn std::error::Error>> {
        todo!()
    }

    async fn move_to(&self, object_id: crate::interfaces::filesystem::ObjectId, new_parent_id: crate::interfaces::filesystem::ObjectId) -> Result<(), Box<dyn std::error::Error>> {
        todo!()
    }

    async fn rename(&self, object_id: crate::interfaces::filesystem::ObjectId, new_name: String) -> Result<(), Box<dyn std::error::Error>> {
        todo!()
    }

    async fn list_folder_content(&self, object_id: crate::interfaces::filesystem::ObjectId) -> Result<Vec<crate::interfaces::filesystem::File>, Box<dyn std::error::Error>> {
        todo!()
    }

    async fn create(&self, parent_id: crate::interfaces::filesystem::ObjectId, file: crate::interfaces::filesystem::File) -> Result<(), Box<dyn std::error::Error>> {
        todo!()
    }

    async fn get_metadata(&self, object_id: crate::interfaces::filesystem::ObjectId) -> Result<crate::interfaces::filesystem::Metadata, Box<dyn std::error::Error>> {
        todo!()
    }
}


#[cfg(test)]
mod tests {
    use crate::providers::one_drive::*;

    #[tokio::test]
    async fn one_drive_login_works() {
        let client_id_vec = std::fs::read("./sandbox/onedrive").unwrap();
        let client_id = std::str::from_utf8(&client_id_vec).unwrap();
        let mut onedrive = OneDrive::new(None, client_id.to_string());
        onedrive.fetch_credentials().unwrap();
        println!("{:?}", onedrive.token);
    }
}