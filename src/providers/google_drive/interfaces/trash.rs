use async_trait::async_trait;

use crate::{interfaces::trash::Trash, providers::google_drive::GoogleDrive};

#[async_trait]
impl Trash for GoogleDrive {
    async fn send_to_trash(&self, object_id: crate::interfaces::filesystem::ObjectId) -> Result<(), Box<dyn std::error::Error>> {
        todo!()
    }
}