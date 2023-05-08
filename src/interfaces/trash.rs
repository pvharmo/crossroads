use async_trait::async_trait;

use super::filesystem::ObjectId;

#[async_trait]
pub trait Trash {
    async fn send_to_trash(&self, object_id: ObjectId) -> Result<(), Box<dyn std::error::Error>>;
}