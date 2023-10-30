use async_trait::async_trait;
use reqwest::StatusCode;

use crate::{interfaces::filesystem::{FileSystem, ObjectId, File, Metadata}, providers::onedrive::OneDrive};

use oauth2::TokenResponse;

use onedrive_api::{OneDrive as OneDriveApi, DriveLocation, ItemId, ItemLocation, FileName, option::DriveItemPutOption, resource::DriveItem};

#[async_trait]
impl FileSystem for OneDrive {
    async fn read_file(&self, object_id: ObjectId) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let drive = OneDriveApi::new(
            self.token.get().await.clone().unwrap().access_token().secret(), // Login token to Microsoft Graph.
            DriveLocation::me(),
        );

        let item_id : ItemId = object_id.clone().into();

        let item_location = if object_id.to_string() == "".to_string() { ItemLocation::root() } else { ItemLocation::from_id(&item_id) };

        let items_result = drive.get_item(item_location).await;

        let item = match items_result {
            Ok(items) => Ok(items),
            Err(error) => {
                if error.status_code() == Some(StatusCode::UNAUTHORIZED) {
                    self.refresh_token().await?;
                    let drive = OneDriveApi::new(
                        self.token.get().await.clone().unwrap().access_token().secret(), // Login token to Microsoft Graph.
                        DriveLocation::me(),
                    );
                    Ok(drive.get_item(item_location).await.unwrap())
                } else {
                    Err(error)
                }
            }
        }?;

        Ok(Vec::from(item.content.unwrap().as_str().as_deref().unwrap().as_bytes()))
    }

    async fn write_file(&self, object_id: ObjectId, content: Vec<u8>) -> Result<(), Box<dyn std::error::Error>> {
        let drive = OneDriveApi::new(
            self.token.get().await.clone().unwrap().access_token().secret(), // Login token to Microsoft Graph.
            DriveLocation::me(),
        );

        let item_id : ItemId = object_id.clone().into();

        let item_location = if object_id.to_string() == "".to_string() { ItemLocation::root() } else { ItemLocation::from_id(&item_id) };

        let mut options = DriveItemPutOption::new();
        options = options.conflict_behavior(onedrive_api::ConflictBehavior::Replace);

        let upload_session_result = drive.new_upload_session_with_option(item_location, options).await;

        let upload_session = match upload_session_result {
            Ok(items) => Ok(items),
            Err(error) => {
                if error.status_code() == Some(StatusCode::UNAUTHORIZED) {
                    self.refresh_token().await?;
                    let drive = OneDriveApi::new(
                        self.token.get().await.clone().unwrap().access_token().secret(), // Login token to Microsoft Graph.
                        DriveLocation::me(),
                    );
                    Ok(drive.new_upload_session(item_location).await?)
                } else {
                    Err(error)
                }
            }
        }?;

        // the size of each byte range MUST be a multiple of 320 KiB
        let chunk_size: u64 = 327_680;
        let content_len: u64 = content.len().try_into().unwrap();
        let chunks = content.chunks_exact(chunk_size.try_into().unwrap());
        let chunks_len: u64 = chunks.len().try_into().unwrap();
        let remainder = chunks.remainder().to_vec();

        println!("{}", chunks.len());

        let client = reqwest::Client::new();

        for (i, chunk) in chunks.enumerate() {
            let index: u64 = i.try_into().unwrap();
            println!("Sending chunk {}", i);

            let request = client.put(upload_session.0.upload_url())
                .header("Content-Length", chunk_size)
                .header("Content-Range", format!("bytes {}-{}/{}", index*chunk_size, (index+1)*chunk_size-1, content_len))
                .body(chunk.to_vec())
                .bearer_auth(self.token.get().await.clone().unwrap().access_token().secret());

            request.send().await?;
        }

        let request = client.put(upload_session.0.upload_url())
                .header("Content-Length", remainder.len())
                .header("Content-Range", format!("bytes {}-{}/{}", chunks_len*chunk_size, content_len-1, content_len))
                .body(remainder)
                .bearer_auth(self.token.get().await.clone().unwrap().access_token().secret());

            request.send().await?;

        Ok(())
    }

    async fn delete(&self, object_id: ObjectId) -> Result<(), Box<dyn std::error::Error>> {
        let drive = OneDriveApi::new(
            self.token.get().await.clone().unwrap().access_token().secret(), // Login token to Microsoft Graph.
            DriveLocation::me(),
        );

        let item_id : ItemId = object_id.clone().into();

        let item_location = if object_id.to_string() == "".to_string() { ItemLocation::root() } else { ItemLocation::from_id(&item_id) };

        let request_result = drive.delete(item_location).await;

        match request_result {
            Ok(items) => Ok(items),
            Err(error) => {
                if error.status_code() == Some(StatusCode::UNAUTHORIZED) {
                    self.refresh_token().await?;
                    let drive = OneDriveApi::new(
                        self.token.get().await.clone().unwrap().access_token().secret(), // Login token to Microsoft Graph.
                        DriveLocation::me(),
                    );
                    Ok(drive.delete(item_location).await.unwrap())
                } else {
                    Err(error)
                }
            }
        }?;

        Ok(())
    }

    async fn move_to(&self, object_id: ObjectId, new_parent_id: ObjectId) -> Result<ObjectId, Box<dyn std::error::Error>> {
        let drive = OneDriveApi::new(
            self.token.get().await.clone().unwrap().access_token().secret(), // Login token to Microsoft Graph.
            DriveLocation::me(),
        );

        let item_id : ItemId = object_id.clone().into();
        let item_location = if object_id.to_string() == "".to_string() { ItemLocation::root() } else { ItemLocation::from_id(&item_id) };

        let parent_item_id : ItemId = new_parent_id.clone().into();
        let parent_location = if object_id.to_string() == "".to_string() { ItemLocation::root() } else { ItemLocation::from_id(&parent_item_id) };

        let request_result = drive.move_(item_location, parent_location, None).await;

        let item = match request_result {
            Ok(items) => Ok(items),
            Err(error) => {
                if error.status_code() == Some(StatusCode::UNAUTHORIZED) {
                    self.refresh_token().await?;
                    let drive = OneDriveApi::new(
                        self.token.get().await.clone().unwrap().access_token().secret(), // Login token to Microsoft Graph.
                        DriveLocation::me(),
                    );
                    Ok(drive.move_(item_location, parent_location, None).await.unwrap())
                } else {
                    Err(error)
                }
            }
        }?;

        Ok(ObjectId::new(item.id.unwrap().as_str().to_string(), object_id.file_type()))
    }

    async fn rename(&self, object_id: ObjectId, new_name: String) -> Result<ObjectId, Box<dyn std::error::Error>> {
        let drive = OneDriveApi::new(
            self.token.get().await.clone().unwrap().access_token().secret(), // Login token to Microsoft Graph.
            DriveLocation::me(),
        );

        let item_id : ItemId = object_id.clone().into();
        let item_location = if object_id.to_string() == "".to_string() { ItemLocation::root() } else { ItemLocation::from_id(&item_id) };

        // let items_result = drive.get_item(item_location).await;

        // let mut item = match items_result {
        //     Ok(items) => Ok(items),
        //     Err(error) => {
        //         if error.status_code() == Some(StatusCode::UNAUTHORIZED) {
        //             self.refresh_token().await?;
        //             let drive = OneDriveApi::new(
        //                 self.token.get().await.clone().unwrap().access_token().secret(), // Login token to Microsoft Graph.
        //                 DriveLocation::me(),
        //             );
        //             Ok(drive.get_item(item_location).await.unwrap())
        //         } else {
        //             Err(error)
        //         }
        //     }
        // }?;

        let mut item = DriveItem::default();

        item.name = Some(new_name);

        let request_result = drive.update_item(item_location, &item).await;

        let item: DriveItem = match request_result {
            Ok(item) => Ok(item),
            Err(error) => {
                if error.status_code() == Some(StatusCode::UNAUTHORIZED) {
                    self.refresh_token().await?;
                    let drive = OneDriveApi::new(
                        self.token.get().await.clone().unwrap().access_token().secret(), // Login token to Microsoft Graph.
                        DriveLocation::me(),
                    );
                    Ok(drive.update_item(item_location, &item).await.unwrap())
                } else {
                    Err(error)
                }
            }
        }?;

        Ok(ObjectId::new(item.id.unwrap().as_str().to_string(), object_id.file_type()))
    }

    async fn read_directory(&self, object_id: ObjectId) -> Result<Vec<File>, Box<dyn std::error::Error>> {
        let drive = OneDriveApi::new(
            self.token.get().await.clone().unwrap().access_token().secret(), // Login token to Microsoft Graph.
            DriveLocation::me(),
        );

        let item_id : ItemId = object_id.clone().into();

        let item_location = if object_id.to_string() == "".to_string() { ItemLocation::root() } else { ItemLocation::from_id(&item_id) };

        let items_result = drive.list_children(item_location).await;

        let items = match items_result {
            Ok(items) => items,
            Err(error) => {
                if error.status_code() == Some(StatusCode::UNAUTHORIZED) {
                    self.refresh_token().await?;
                    let drive = OneDriveApi::new(
                        self.token.get().await.clone().unwrap().access_token().secret(), // Login token to Microsoft Graph.
                        DriveLocation::me(),
                    );
                    drive.list_children(item_location).await.unwrap()
                } else {
                    vec![]
                }
            }
        };

        let files: Vec<File> = items.iter().map(|file| file.to_owned().into()).collect();

        Ok(files)
    }

    async fn create(&self, parent_id: ObjectId, file: File) -> Result<(), Box<dyn std::error::Error>> {
        let drive = OneDriveApi::new(
            self.token.get().await.clone().unwrap().access_token().secret(), // Login token to Microsoft Graph.
            DriveLocation::me(),
        );

        let item_id : ItemId = parent_id.clone().into();

        let item_location = if parent_id.to_string() == "".to_string() { ItemLocation::root() } else { ItemLocation::from_id(&item_id) };

        let filename = FileName::new(&file.name);

        if file.id.is_directory() {
            println!("Creating a directory");
            let items_result = drive.create_folder(item_location, filename.unwrap()).await;
    
            match items_result {
                Ok(items) => Ok(items),
                Err(error) => {
                    if error.status_code() == Some(StatusCode::UNAUTHORIZED) {
                        self.refresh_token().await?;
                        let drive = OneDriveApi::new(
                            self.token.get().await.clone().unwrap().access_token().secret(), // Login token to Microsoft Graph.
                            DriveLocation::me(),
                        );
                        Ok(drive.get_item(item_location).await.unwrap())
                    } else {
                        println!("Got a different error");
                        Err(error)
                    }
                }
            }?;

            return Ok(())
        } else {
            println!("Creating a file");

            let client = reqwest::Client::new();

            let response = client.put(format!("https://graph.microsoft.com/v1.0/me/drive/items/{}:/{}:/content", parent_id.as_str(), file.name.as_str()))
                .header("Content-Type", "text/plain").header("Content-Length", "0").body("")
                .bearer_auth(self.token.get().await.clone().unwrap().access_token().secret()).send().await;
    
            let result = match response {
                Ok(items) => Ok(items),
                Err(error) => {
                    if error.status() == Some(StatusCode::UNAUTHORIZED) {
                        self.refresh_token().await?;
                        let retry_response = client.put(format!("https://graph.microsoft.com/v1.0/me/drive/items/{}:/{}:/content", parent_id.as_str(), file.name.as_str()))
                            .header("Content-Type", "text/plain").body("")
                            .bearer_auth(self.token.get().await.clone().unwrap().access_token().secret()).send().await?;
                        Ok(retry_response)
                    } else {
                        Err(error)
                    }
                }
            }?;

            println!("{}", result.text().await.unwrap());

            Ok(())
        }
    }

    async fn get_metadata(&self, object_id: ObjectId) -> Result<crate::interfaces::filesystem::Metadata, Box<dyn std::error::Error>> {
        let drive = OneDriveApi::new(
            self.token.get().await.clone().unwrap().access_token().secret(), // Login token to Microsoft Graph.
            DriveLocation::me(),
        );

        let item_id : ItemId = object_id.clone().into();

        let item_location = if object_id.to_string() == "".to_string() { ItemLocation::root() } else { ItemLocation::from_id(&item_id) };

        let items_result = drive.get_item(item_location).await;

        let item = match items_result {
            Ok(items) => Ok(items),
            Err(error) => {
                if error.status_code() == Some(StatusCode::UNAUTHORIZED) {
                    self.refresh_token().await?;
                    let drive = OneDriveApi::new(
                        self.token.get().await.clone().unwrap().access_token().secret(), // Login token to Microsoft Graph.
                        DriveLocation::me(),
                    );
                    Ok(drive.get_item(item_location).await.unwrap())
                } else {
                    Err(error)
                }
            }
        }?;

        Ok(Metadata {
            mime_type: None,
            open_path: Some(item.web_url.unwrap()),
            modified_at: None,
            created_at: None,
            meta_changed_at: None,
            accessed_at: None,
            size: None,
            owner: None,
            permissions: None,
        })
    }

    async fn read_link(&self, object_id: ObjectId) -> Result<ObjectId, Box<dyn std::error::Error>> {
        todo!()
    }

    async fn create_link(&self, parent_id: ObjectId, name: &str, link_id: ObjectId) -> Result<ObjectId, Box<dyn std::error::Error>> {
        todo!()
    }
}
