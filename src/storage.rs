use crate::interfaces::Provider;
use crate::interfaces::filesystem::{FileSystem, File, ObjectId};
use crate::providers::onedrive::OneDrive;
use crate::providers::onedrive::token::OneDriveToken;
use crate::providers::s3::S3Credentials;
use crate::providers::{s3::S3, google_drive::GoogleDrive, native_fs::NativeFs};
use google_drive3::oauth2::storage::TokenInfo;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::hash::Hash;
use std::str::FromStr;
use directories::ProjectDirs;
use std::sync::Arc;

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Clone)]
struct ArbitraryData(serde_json::Value);

impl Hash for ArbitraryData {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.as_str().hash(state);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProvidersOptions {
    pub google_api_key: Option<String>,
    pub onedrive_api_key: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Hash, Eq, PartialEq, Clone, Copy)]
pub enum ProviderType {
    GoogleDrive,
    OneDrive,
    S3,
    NativeFs,
}

impl FromStr for ProviderType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "googledrive" => Ok(ProviderType::GoogleDrive),
            "onedrive" => Ok(ProviderType::OneDrive),
            "s3" => Ok(ProviderType::S3),
            "nativefs" => Ok(ProviderType::NativeFs),
            _ => Err(())
        }
    }
}

impl fmt::Display for ProviderType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Serialize, Deserialize, Hash, Eq, PartialEq, Clone)]
pub struct ProviderId {
    pub id: String,
    pub provider_type: ProviderType,
}

pub struct ProvidersMap {
    providers : HashMap<ProviderId, Arc<dyn Provider + Sync + Send>>,
    keys : ProvidersOptions
}

impl ProvidersMap {
    pub async fn new(keys: ProvidersOptions) -> ProvidersMap {
        let providers: HashMap<ProviderId, Arc<dyn Provider + Sync + Send>> = HashMap::new();

        ProvidersMap {providers, keys}
    }

    pub fn get_provider(& self, provider: ProviderId) -> Result<Arc<dyn Provider>, String> {
        match self.providers.get(&provider) {
            Some(x) => Ok(x.clone()),
            None => Err(String::from("Provider not found"))
        }
    }

    pub async fn add_google_drive(&mut self, provider_id: ProviderId, tokens: HashMap<String, TokenInfo>) -> Result<(), ()> {
        dbg!(&tokens);
        let google_drive = GoogleDrive::new(self.keys.google_api_key.clone().unwrap().to_string(), tokens).await.unwrap();
        google_drive.list_folder_content(ObjectId::directory("".to_string())).await.unwrap();

        self.save(&provider_id, serde_json::to_value(&google_drive.tokens_map()).unwrap()).await;
        self.providers.insert(provider_id.clone(), Arc::new(google_drive));

        Ok(())
    }

    pub async fn add_onedrive(&mut self, provider_id: ProviderId, token: Option<OneDriveToken>) -> Result<(), ()> {
        let should_fetch_credentials = token.is_none();
        let onedrive = OneDrive::new(token, self.keys.onedrive_api_key.clone().unwrap().to_string());

        if should_fetch_credentials {
            onedrive.fetch_credentials().await.unwrap();
        }

        self.save(&provider_id, serde_json::to_value(&onedrive.get_token().await).unwrap()).await;
        self.providers.insert(provider_id.clone(), Arc::new(onedrive));

        Ok(())
    }

    pub async fn add_s3(&mut self, provider_id: ProviderId, bucket: String, credentials: S3Credentials) -> Result<(), ()> {
        let s3 = S3::new(bucket, credentials);

        self.save(&provider_id, serde_json::to_value(&s3).unwrap()).await;
        self.providers.insert(provider_id.clone(), Arc::new(s3));

        Ok(())
    }

    pub async fn add_native_fs(&mut self, provider_id: ProviderId, root: String) -> Result<(), ()> {
        let native_fs = NativeFs { root: root.clone() };

        self.save(&provider_id, serde_json::to_value(&native_fs).unwrap()).await;
        self.providers.insert(provider_id.clone(), Arc::new(native_fs));
        
        Ok(())
    }

    pub async fn save(&mut self, provider_id: &ProviderId, value: serde_json::Value) {
        let storage = NativeFs { root : "".to_string() };
        if let Some(proj_dirs) = ProjectDirs::from("", "Orbital", "Files") {
            let path = (proj_dirs.data_dir().to_string_lossy() + "/").to_string();
    
            let file_name = provider_id.id.clone() + "." + provider_id.provider_type.to_string().as_str();
    
            let file: File = File {
                id: ObjectId::plain_text(path.clone() + file_name.as_str()),
                name: file_name.clone(),
                mime_type: Some("text/plain".to_string()),
                created_at: Some(chrono::Utc::now()),
                modified_at: Some(chrono::Utc::now()),
                size: None
            };
    
            storage.create(ObjectId::plain_text(path.clone()), file.clone()).await.expect(format!("Unable to create provider {}", path.clone() + file_name.as_str()).as_str());
    
            storage.write_file(file.id, value.to_string().as_bytes().to_vec()).await.expect("Unable to write new provider to storage");
        }
    }

    pub async fn add_provider(&mut self, provider_id: ProviderId, provider_infos: serde_json::Value) -> Result<(), ()> {
        println!("----");
        match provider_id.provider_type {
            ProviderType::NativeFs => {
                let root: String = serde_json::from_value(provider_infos).unwrap();
                self.add_native_fs(provider_id, root).await.unwrap();
            },
            ProviderType::GoogleDrive => {
                if let Ok(tokens) = serde_json::from_value(provider_infos) {
                    self.add_google_drive(provider_id, tokens).await.unwrap();
                } else {
                    self.add_google_drive(provider_id, HashMap::new()).await.unwrap();
                }
            },
            ProviderType::OneDrive => {
                if let Ok(token) = serde_json::from_value(provider_infos) {
                    self.add_onedrive(provider_id, token).await.unwrap();
                } else {
                    self.add_onedrive(provider_id, None).await.unwrap();
                }
            },
            ProviderType::S3 => {
                let credentials : S3Credentials = serde_json::from_value(provider_infos.get("credentials").unwrap().to_owned()).unwrap();
                let bucket : String = serde_json::from_value(provider_infos.get("bucket").unwrap().to_owned()).unwrap();
                self.add_s3(provider_id, bucket, credentials).await.unwrap();
            },
            _ => { return Err(()) }
        };

        Ok(())
    }

    pub async fn remove_provider(&self, provider_id: ProviderId) -> ProvidersMap {
        if let Some(proj_dirs) = ProjectDirs::from("", "Orbital", "Files") {
            let storage = NativeFs { root : "".to_string() };
            let path = (proj_dirs.data_dir().to_string_lossy() + "/").to_string();
    
            storage.delete(ObjectId::plain_text(path + provider_id.id.as_str() + "." + provider_id.provider_type.to_string().as_str())).await.expect("Unable to remove provider.");
        }

        ProvidersMap::new(self.keys.clone()).await
    }

    pub fn list_providers(&self) -> Vec<ProviderId> {
        let mut providers_vec: Vec<ProviderId> = Vec::new();
        
        for provider in self.providers.keys() {
            providers_vec.push((*provider).clone());
        }

        providers_vec
    }
}