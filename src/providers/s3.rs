use std::str::FromStr;

use async_trait::async_trait;
use eyre::Result;
use s3::{creds::Credentials, bucket::Bucket};
use serde::{Serialize, Deserialize};

use crate::interfaces::{filesystem::{FileSystem, ObjectId, File}, Provider};


#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct S3Credentials {
    pub region: String,
    pub endpoint: String,
    pub access_key: String,
    pub secret_key: String
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct S3 {
    pub credentials: S3Credentials,
    pub bucket: String,
}

impl S3 {
    pub fn new(bucket: String, credentials: S3Credentials) -> S3 {
        S3 { credentials, bucket }
    }
}

impl Provider for S3 {
    fn as_filesystem(&self) -> Option<&dyn crate::interfaces::filesystem::FileSystem> {
        Some(self)
    }

    fn as_trash(&self) -> Option<&dyn crate::interfaces::trash::Trash> {
        None
    }
}

#[async_trait]
impl FileSystem for S3 {
    async fn read_file(&self, object_id: ObjectId) -> Result<Vec<u8>, Box<dyn std::error::Error>>
    {
        let mut bucket = Bucket::new(
            self.bucket.as_str(),
            s3::region::Region::Custom { region: self.credentials.region.clone(), endpoint: self.credentials.endpoint.clone() },
            Credentials { 
                access_key: Some(self.credentials.access_key.clone()),
                secret_key: Some(self.credentials.secret_key.clone()),
                security_token: None, session_token: None, expiration: None
            }
        )?;

        bucket.set_path_style();

        let val = bucket.get_object(object_id.to_string())?;

        Ok(val.bytes().to_vec())
    }

    async fn write_file(&self, _object_id: ObjectId, _content: Vec<u8>) -> Result<(), Box<dyn std::error::Error>> {
        todo!()
    }

    async fn delete(&self, _object_id: ObjectId) -> Result<(), Box<dyn std::error::Error>> {
        todo!()
    }

    async fn create(&self, _parent_id: ObjectId, _file: File) -> Result<(), Box<dyn std::error::Error>> {
        todo!()
    }

    async fn rename(&self, object_id: ObjectId, new_name: String) -> Result<(), Box<dyn std::error::Error>> {
        let path = match object_id.to_string().strip_prefix("/") {
            Some(x) => x.to_string(),
            None => object_id.to_string()
        };

        let new_path = match new_name.strip_prefix("/") {
            Some(x) => x.to_string(),
            None => new_name
        };

        let mut bucket = Bucket::new(
            self.bucket.as_str(),
            s3::region::Region::Custom { region: self.credentials.region.clone(), endpoint: self.credentials.endpoint.clone() },
            Credentials { 
                access_key: Some(self.credentials.access_key.clone()),
                secret_key: Some(self.credentials.secret_key.clone()),
                security_token: None, session_token: None, expiration: None
            }
        )?;

        bucket.set_path_style();

        bucket.copy_object_internal(&path, new_path)?;
        bucket.delete_object(path)?;

        Ok(())
    }

    async fn move_to(&self, _object_id: ObjectId, _new_parent_id: ObjectId) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }

    async fn list_folder_content(&self, object_id: ObjectId) -> Result<Vec<File>, Box<dyn std::error::Error>> {
        let path = match object_id.to_string().strip_prefix("/") {
            Some(x) => x.to_string(),
            None => object_id.to_string()
        };

        let mut bucket = Bucket::new(
            self.bucket.as_str(),
            s3::region::Region::Custom { region: self.credentials.region.clone(), endpoint: self.credentials.endpoint.clone() },
            Credentials { 
                access_key: Some(self.credentials.access_key.clone()),
                secret_key: Some(self.credentials.secret_key.clone()),
                security_token: None, session_token: None, expiration: None
            }
        )?;

        bucket.set_path_style();

        let buckets = bucket.list(path.to_string(), None)?;

        let mut files = vec![];

        for bucket in buckets {
            for file in bucket.contents {
                let x = (&file).key.strip_prefix(&path);
                let z = match x {
                    Some(y) => y.to_string(),
                    None => (&file).key.clone()
                };
                let w = match z.strip_prefix("/") {
                    Some(a) => a.to_string(),
                    None => z
                };
                let name_split = w.splitn(2, "/").next();
                match name_split {
                    Some(name) => {
                        let mime_type = if w.contains("/") {"directory"} else {"file"};
                        files.push(File {
                            id: ObjectId::new(path.to_string() + "/" + &name, None),
                            name: name.to_string(),
                            mime_type: Some(mime_type.to_string()),
                            created_at: None,
                            modified_at: Some(chrono::DateTime::from_str(file.last_modified.as_str()).unwrap()),
                            size: Some(file.size)
                        })
                    },
                    None => ()
                };
                
            }
        }

        files.dedup();

        Ok(files)
    }

    async fn get_metadata(&self, _object_id: ObjectId) -> Result<crate::interfaces::filesystem::Metadata, Box<dyn std::error::Error>> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use crate::providers::s3::*;
    use crate::interfaces::filesystem::FileSystem;

    #[tokio::test]
    async fn s3_request_works() {
        let x = S3 {
            credentials: S3Credentials {
                access_key: String::from("admin"),
                secret_key: String::from("password"),
                region: String::from(""),
                endpoint: String::from("http://localhost:9000")
            },
            bucket: String::from("test")
        };
        let result = x.read_file(ObjectId::new(String::from("hello-world.txt"), Some(String::from("text/plain")))).await;
        assert!(result.is_ok());
        assert_eq!(String::from_utf8(result.unwrap().to_vec()).unwrap(), String::from("hello world!"));
    }

    #[tokio::test]
    async fn s3_list_folder_content() {
        let x = S3 {
            credentials: S3Credentials {
                access_key: String::from("admin"),
                secret_key: String::from("password"),
                region: String::from(""),
                endpoint: String::from("http://localhost:9000")
            },
            bucket: String::from("test")
        };

        let result = x.list_folder_content(ObjectId::new(String::from("/"), Some(String::from("directory")))).await;

        assert!(result.is_ok());

        assert!(result.as_ref().unwrap().len() == 2);

        assert_eq!("hello-world.txt", result.as_ref().unwrap()[0].name);
        assert_eq!("level1", result.as_ref().unwrap()[1].name);
    }

    #[tokio::test]
    async fn s3_list_folder_content_one_level_deep() {
        let x = S3 {
            credentials: S3Credentials {
                access_key: String::from("admin"),
                secret_key: String::from("password"),
                region: String::from(""),
                endpoint: String::from("http://localhost:9000")
            },
            bucket: String::from("test")
        };

        let result = x.list_folder_content(ObjectId::new(String::from("/level1/"), Some(String::from("directory")))).await;

        assert!(result.is_ok());

        assert!(result.as_ref().unwrap().len() == 2);

        assert_eq!("level1_file.txt", result.as_ref().unwrap()[0].name);
        assert_eq!("level2", result.as_ref().unwrap()[1].name);
    }

    #[tokio::test]
    async fn s3_rename_file() {
        let x = S3 {
            credentials: S3Credentials {
                access_key: String::from("admin"),
                secret_key: String::from("password"),
                region: String::from(""),
                endpoint: String::from("http://localhost:9000")
            },
            bucket: String::from("test")
        };

        let original_id = ObjectId::new(String::from("/test.txt"), Some(String::from("text/plain")));

        let result = x.rename(original_id, String::from("/test_renamed.txt")).await;

        assert!(result.is_ok());

        let new_id = ObjectId::new(String::from("/test_renamed.txt"), Some(String::from("text/plain")));

        let file = x.read_file(new_id.clone()).await;

        assert!(file.is_ok());

        let file_unwrapped = file.unwrap();

        let content = std::str::from_utf8(&file_unwrapped);
        assert!(content.is_ok());
        dbg!(content.unwrap());
        assert!(content.unwrap() == "Lorem ipsum\n");

        let reverse_rename = x.rename(new_id, String::from("/test.txt")).await;

        assert!(reverse_rename.is_ok());
    }
}