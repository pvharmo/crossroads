extern crate google_drive3 as drive3;
use async_trait::async_trait;
use drive3::api::File as GoogleDriveFile;

use crate::interfaces::filesystem::{FileSystem, ObjectId, File, Metadata, FileType, self};

use super::super::GoogleDrive;

impl From<google_drive3::api::File> for filesystem::File {
    fn from(file: GoogleDriveFile) -> Self {
        let mime_type = file.mime_type.clone();
        let id;

        if mime_type.clone().unwrap_or_default() == "application/vnd.google-apps.folder" {
            id = ObjectId::directory(file.id.unwrap());
        } else {
            id = ObjectId::new(file.id.unwrap(), FileType::File);
        }
        File {
            id,
            name: file.name.unwrap(),
            metadata: Some(Metadata {
                mime_type: file.mime_type,
                created_at: file.created_time,
                modified_at: file.modified_time,
                meta_changed_at: None,
                accessed_at: None,
                size: Some(file.size.unwrap_or(0).unsigned_abs()),
                open_path: None,
                owner: None,
                permissions: None
            })
        }
    }
}

#[async_trait]
impl FileSystem for GoogleDrive {
    async fn read_file(&self, object_id: ObjectId) -> Result<Vec<u8>, Box<dyn std::error::Error>>
    {
        todo!()
    }

    async fn write_file(&self, object_id: ObjectId, content: Vec<u8>) -> Result<(), Box<dyn std::error::Error>> {
        todo!()
    }

    async fn delete(&self, object_id: ObjectId) -> Result<(), Box<dyn std::error::Error>> {
        todo!()
    }

    async fn rename(&self, object_id: ObjectId, new_name: String) -> Result<ObjectId, Box<dyn std::error::Error>> {
        todo!()
    }

    async fn move_to(&self, object_id: ObjectId, new_parent_id: ObjectId) -> Result<ObjectId, Box<dyn std::error::Error>> {
        todo!()
    }

    async fn create(&self, parent_id: ObjectId, file: File) -> Result<(), Box<dyn std::error::Error>> {
        todo!()
    }

    async fn read_directory(&self, object_id: ObjectId) -> Result<Vec<File>, Box<dyn std::error::Error>> {
        let id = if object_id.to_string() == "".to_string() {"root".to_string()} else {object_id.to_string()};
        let response = self.hub.files().list().q(format!("'{}' in parents", id).as_str()).doit().await?;
        
        let files: Vec<File> = response.1.files.unwrap().iter().map(|file| file.to_owned().into()).collect();
        
        Ok(files)
    }

    async fn get_metadata(&self, object_id: ObjectId) -> Result<Metadata, Box<dyn std::error::Error>> {
        todo!()
    }

    async fn read_link(&self, object_id: ObjectId) -> Result<ObjectId, Box<dyn std::error::Error>> {
        todo!()
    }

    async fn create_link(&self, parent_id: ObjectId, name: &str, link_id: ObjectId) -> Result<ObjectId, Box<dyn std::error::Error>> {
        todo!()
    }
}