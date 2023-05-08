use oauth2::basic::{BasicClient, BasicTokenType, BasicErrorResponseType};
use oauth2::reqwest::async_http_client;
use oauth2::{
    AuthType, AuthUrl, AuthorizationCode, ClientId, CsrfToken, PkceCodeChallenge,
    RedirectUrl, Scope, TokenUrl, StandardTokenResponse, EmptyExtraTokenFields, PkceCodeVerifier, TokenResponse, StandardErrorResponse, Client, StandardTokenIntrospectionResponse, StandardRevocableToken, RevocationErrorResponseType,
};
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use oauth2::url::Url;

use super::token::{TokenStorage, OneDriveToken};
use super::{OneDrive};

async fn listen_for_token(client: BasicClient, csrf_state: CsrfToken, pkce_code_verifier: PkceCodeVerifier) -> Result<StandardTokenResponse<EmptyExtraTokenFields, BasicTokenType>, ()> {
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
                .request_async(async_http_client).await;

            if token_result.is_err() {
                println!("There was an error");
                dbg!(token_result.err());
                return Err(())
            }
            
            println!("It's ok");
            return Ok(token_result.unwrap())
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

    pub async fn fetch_credentials(&self) -> Result<(), Box<dyn std::error::Error>> {
        let client = Self::new_client(self.client_id.clone());
    
        // Microsoft Graph supports Proof Key for Code Exchange (PKCE - https://oauth.net/2/pkce/).
        // Create a PKCE code verifier and SHA-256 encode it as a code challenge.
        let (pkce_code_challenge, pkce_code_verifier) = PkceCodeChallenge::new_random_sha256();
    
        // Generate the authorization URL to which we'll redirect the user.
        let (authorize_url, csrf_state) = client
            .authorize_url(CsrfToken::new_random)
            // This example requests read access to OneDrive.
            .add_scope(Scope::new(
                "https://graph.microsoft.com/Files.ReadWrite.All".to_string()
            ))
            .add_scope(Scope::new("offline_access".to_string()))
            .set_pkce_challenge(pkce_code_challenge)
            .url();
    
        println!(
            "Open this URL in your browser:\n{}\n",
            authorize_url.to_string()
        );

        open::that(authorize_url.to_string()).expect("Could not open browser to authenticate to OneDrive.");

        let token = listen_for_token(client, csrf_state, pkce_code_verifier).await.expect("Error fetching OneDrive access token");

        self.token.set(Some(token)).await;

        Ok(())
    }

    pub fn new (token: Option<OneDriveToken>, client_id: String) -> OneDrive {
        OneDrive { token: TokenStorage::new(token), client_id }
    }

    pub async fn refresh_token(&self) -> Result<(), Box<dyn std::error::Error>> {
        let client = Self::new_client(self.client_id.clone());
        if let Some(refresh_token) = self.token.get().await.clone().unwrap().refresh_token() {
            let token = client.exchange_refresh_token(refresh_token).request_async(async_http_client).await?;
            self.token.set(Some(token)).await;
        } else {
            self.fetch_credentials().await?;
        }

        Ok(())
    }

    pub async fn get_token(&self) -> Option<OneDriveToken> {
        self.token.get().await
    }
}