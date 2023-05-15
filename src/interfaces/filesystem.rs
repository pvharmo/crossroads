use std::fmt;

use async_trait::async_trait;
use serde::{Serialize, Deserialize};
use chrono::prelude::{DateTime, Utc};

#[derive(Serialize, Deserialize, Debug, Hash, PartialEq, Eq, Clone)]
pub enum FileType {
    Directory,
    File,
    Symlink
}

#[derive(Serialize, Deserialize, Debug, Hash, PartialEq, Eq, Clone)]
pub struct ObjectId {
    path: String,
    file_type: FileType
}

impl fmt::Display for ObjectId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.path)
    }
}

impl ObjectId {
    pub fn new(path: String, file_type: FileType) -> Self {
        ObjectId { path, file_type }
    }

    pub fn root() -> Self {
        ObjectId { path: "".to_string(), file_type: FileType::Directory }
    }

    pub fn directory(path: String) -> Self {
        ObjectId { path, file_type: FileType::Directory }
    }

    pub fn plain_text(path: String) -> Self {
        ObjectId { path, file_type: FileType::File }
    }

    pub fn as_str(&self) -> &str {
        self.path.as_str()
    }

    pub fn file_type(&self) -> FileType {
        self.file_type.clone()
    }

    pub fn is_directory(&self) -> bool {
        self.file_type == FileType::Directory
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct File {
    pub id: ObjectId,
    pub name: String,
    pub metadata: Option<Metadata>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub enum Permissions {
    Unix(u32)
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub enum UserId {
    UserAndGroup(u32, u32),
    UniqueId(String),
    NotApplicable
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct  User {
    pub id: UserId,
    pub name: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone, Default)]
pub struct Metadata {
    pub mime_type: Option<String>,
    pub open_path: Option<String>,
    pub modified_at: Option<DateTime<Utc>>,
    pub created_at: Option<DateTime<Utc>>,
    pub meta_changed_at: Option<DateTime<Utc>>,
    pub accessed_at: Option<DateTime<Utc>>,
    pub size: Option<u64>,
    pub owner: Option<User>,
    pub permissions: Option<Permissions>
}

#[async_trait]
pub trait FileSystem {
    async fn read_file(&self, object_id: ObjectId) -> Result<Vec<u8>, Box<dyn std::error::Error>>;
    async fn write_file(&self, object_id: ObjectId, content: Vec<u8>) -> Result<(), Box<dyn std::error::Error>>;
    async fn delete(&self, object_id: ObjectId) -> Result<(), Box<dyn std::error::Error>>;
    async fn move_to(&self, object_id: ObjectId, new_parent_id: ObjectId) -> Result<ObjectId, Box<dyn std::error::Error>>;
    async fn rename(&self, object_id: ObjectId, new_name: String) -> Result<ObjectId, Box<dyn std::error::Error>>;
    async fn read_directory(&self, object_id: ObjectId) -> Result<Vec<File>, Box<dyn std::error::Error>>;
    async fn create(&self, parent_id: ObjectId, file: File) -> Result<(), Box<dyn std::error::Error>>;
    async fn get_metadata(&self, object_id: ObjectId) -> Result<Metadata, Box<dyn std::error::Error>>;
}