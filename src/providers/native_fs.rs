use async_trait::async_trait;
use chrono::Utc;
use eyre::Result;
use std::fs;
use serde::{Serialize, Deserialize};
use trash;
use std::fs::File as NativeFile;
use std::os::unix::fs::{MetadataExt, PermissionsExt};

use crate::interfaces::filesystem::{User, UserId, Permissions};
use crate::interfaces::{filesystem::{FileSystem, ObjectId, File, Metadata}, Provider, trash::Trash};


#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NativeFs {
    pub root: String,
}

impl NativeFs {
    pub fn new(root: String) -> NativeFs{
      NativeFs {
        root
      }
    }
}

impl Provider for NativeFs {
    fn as_filesystem(&self) -> Option<&dyn crate::interfaces::filesystem::FileSystem> {
        Some(self)
    }

    fn as_trash(&self) -> Option<&dyn crate::interfaces::trash::Trash> {
        Some(self)
    }
}

#[async_trait]
impl FileSystem for NativeFs {
    async fn read_file(&self, object_id: ObjectId) -> Result<Vec<u8>, Box<dyn std::error::Error>>
    {
        let content = fs::read(self.root.clone() + object_id.as_str())?;

        Ok(content)
    }

    async fn write_file(&self, object_id: ObjectId, content: Vec<u8>) -> Result<(), Box<dyn std::error::Error>> {
        fs::write(self.root.clone() + object_id.as_str(), content)?;
        Ok(())
    }

    async fn delete(&self, object_id: ObjectId) -> Result<(), Box<dyn std::error::Error>> {
        if object_id.mime_type() == Some("directory".to_string()) {
            fs::remove_dir(self.root.clone() + object_id.as_str())?;
        } else {
            fs::remove_file(self.root.clone() + object_id.as_str())?;
        }
        Ok(())
    }

    async fn rename(&self, object_id: ObjectId, new_name: String) -> Result<ObjectId, Box<dyn std::error::Error>> {
        let new_path = std::path::Path::new(object_id.as_str()).parent().unwrap().join(new_name);
        fs::rename(self.root.clone() + object_id.as_str(), self.root.clone() + new_path.to_str().unwrap())?;
        Ok(ObjectId::new(new_path.to_str().unwrap().to_string(), object_id.mime_type()))
    }

    async fn move_to(&self, object_id: ObjectId, new_parent_id: ObjectId) -> Result<ObjectId, Box<dyn std::error::Error>> {
        let object_id_split: Vec<&str> = object_id.as_str().split("/").collect();
        let new_path = self.root.clone() + new_parent_id.as_str() + "/" + object_id_split[object_id_split.len() - 1];
        fs::rename(self.root.clone() + object_id.as_str(), new_path.clone())?;
        Ok(ObjectId::new(new_path, object_id.mime_type()))
    }

    async fn create(&self, parent_id: ObjectId, file: File) -> Result<(), Box<dyn std::error::Error>> {
        if file.metadata.unwrap().mime_type == Some("directory".to_string()) {
            fs::create_dir(self.root.clone() + parent_id.as_str() + "/" + file.name.as_str())?;
        } else {
            NativeFile::create(self.root.clone() + parent_id.as_str() + "/" + file.name.as_str())?;
        }
        Ok(())
    }

    async fn read_directory(&self, object_id: ObjectId) -> Result<Vec<File>, Box<dyn std::error::Error>> {
        let dir_content = fs::read_dir(self.root.clone() + object_id.as_str())?;

        let mut files = vec![];

        for file in dir_content {
            let entry = file.unwrap();
            let full_path = entry.path().as_os_str().to_str().unwrap().to_string();
            let mut mime_type = None;
            let mut created_at = None;
            let mut modified_at = None;
            let mut owner = None;
            let mut meta_changed_at = None;
            let mut accessed_at = None;
            let mut permissions = None;
            if let Ok(metadata) = entry.metadata() {
                mime_type = if metadata.is_dir() {
                    Some("directory".to_string())
                } else if metadata.is_symlink() {
                    Some("symlink".to_string())
                } else {
                    Some("text/plain".to_string())
                };

                if let Ok(time) = metadata.created() {
                    created_at = Some(chrono::DateTime::from(time));
                } else {
                    created_at = None;
                }
    
                if let Ok(time) = metadata.modified() {
                    modified_at = Some(chrono::DateTime::from(time));
                } else {
                    modified_at = None;
                }
    
                owner = Some(User {
                    #[cfg(target_family = "unix")]
                    id: UserId::UserAndGroup(metadata.uid(), metadata.gid()),
                    #[cfg(target_family = "windows")]
                    id: UserId::NotApplicable,
                    name: None,
                });
                
                #[cfg(target_family = "unix")]
                {
                    permissions = Some(Permissions::Unix(metadata.permissions().mode()));

                    let ctime = chrono::NaiveDateTime::from_timestamp_opt(metadata.ctime(), 0);
                    if let Some(ctime) = ctime {
                        meta_changed_at = Some(chrono::DateTime::<Utc>::from_utc(ctime, Utc));
                    }

                    let atime = chrono::NaiveDateTime::from_timestamp_opt(metadata.atime(), 0);
                    if let Some(atime) = atime {
                        accessed_at = Some(chrono::DateTime::<Utc>::from_utc(atime, Utc));
                    }
                }
            }

            files.push(File {
                id: ObjectId::new(full_path.strip_prefix(&self.root.clone()).unwrap().to_string(), mime_type.clone()),
                name: entry.file_name().to_string_lossy().to_string(),
                metadata: Some(Metadata {
                    mime_type,
                    created_at,
                    modified_at,
                    meta_changed_at,
                    accessed_at,
                    size: Some(entry.metadata().unwrap().len()),
                    open_path: None,
                    owner,
                    permissions,
                })
            });
        }
        Ok(files)
    }

    async fn get_metadata(&self, object_id: ObjectId) -> Result<crate::interfaces::filesystem::Metadata, Box<dyn std::error::Error>> {
        let metadata = std::fs::metadata(self.root.clone() + object_id.as_str()).unwrap();
        let open_path = Some(self.root.clone() + object_id.as_str());

        metadata.permissions().mode();

        let created_at;
        let modified_at;
        let mut meta_changed_at = None;
        let mut accessed_at = None;
        let mut permissions = None;

        let size = Some(metadata.len());
        let mime_type = if metadata.is_dir() {
            Some("directory".to_string())
        } else if metadata.is_symlink() {
            Some("symlink".to_string())
        } else {
            Some("text/plain".to_string())
        };

        if let Ok(time) = metadata.created() {
            created_at = Some(chrono::DateTime::from(time));
        } else {
            created_at = None;
        }

        if let Ok(time) = metadata.modified() {
            modified_at = Some(chrono::DateTime::from(time));
        } else {
            modified_at = None;
        }

        let owner = Some(User {
            #[cfg(target_family = "unix")]
            id: UserId::UserAndGroup(metadata.uid(), metadata.gid()),
            #[cfg(target_family = "windows")]
            id: UserId::NotApplicable,
            name: None,
        });
        
        #[cfg(target_family = "unix")]
        {
            permissions = Some(Permissions::Unix(metadata.permissions().mode()));

            let ctime = chrono::NaiveDateTime::from_timestamp_opt(metadata.ctime(), 0);
            if let Some(ctime) = ctime {
                meta_changed_at = Some(chrono::DateTime::<Utc>::from_utc(ctime, Utc));
            }

            let atime = chrono::NaiveDateTime::from_timestamp_opt(metadata.atime(), 0);
            if let Some(atime) = atime {
                accessed_at = Some(chrono::DateTime::<Utc>::from_utc(atime, Utc));
            }
        }

        Ok(Metadata {
            modified_at,
            created_at,
            meta_changed_at,
            accessed_at,
            mime_type,
            open_path,
            size,
            owner,
            permissions,
        })
    }
}

#[async_trait]
impl Trash for NativeFs {
    async fn send_to_trash(&self, object_id: crate::interfaces::filesystem::ObjectId) -> Result<(), Box<dyn std::error::Error>> {
        dbg!(self.root.clone() + object_id.as_str());
        trash::delete(self.root.clone() + object_id.as_str()).unwrap();
        Ok(())
    }
}


#[cfg(test)]
mod tests {
    use crate::providers::native_fs::*;
    use crate::interfaces::filesystem::FileSystem;

    #[tokio::test]
    async fn native_fs_request_works() {
        let x = NativeFs {
            root: "./sandbox/".to_string()
        };
        let object_id = ObjectId::new(String::from("hello-world.txt"), Some(String::from("text/plain")));
        let result = x.read_file(object_id).await;
        assert!(result.is_ok());
        assert_eq!(String::from_utf8(result.unwrap().to_vec()).unwrap(), String::from("hello world!"));
    }

    #[tokio::test]
    async fn native_fs_list_folder_content() {
        let x = NativeFs {
            root: "./sandbox/".to_string()
        };

        let object_id = ObjectId::new(String::from(""), Some(String::from("directory")));

        let result = x.read_directory(object_id).await;

        assert!(result.is_ok());

        assert_eq!("hello-world.txt", result.as_ref().unwrap()[0].name);
    }
}