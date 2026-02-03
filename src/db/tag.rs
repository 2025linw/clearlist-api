use uuid::Uuid;

use crate::error::Result;

use super::DatabaseConn;

impl DatabaseConn {
    pub async fn query_tag() -> Result<()> {
        todo!()
    }

    pub async fn create_tag() -> Result<()> {
        todo!()
    }

    pub async fn retrieve_tag() -> Result<()> {
        todo!()
    }

    pub async fn update_tag() -> Result<()> {
        todo!()
    }

    pub async fn delete_tag(tag_id: Uuid, user_id: Uuid) -> Result<()> {
        todo!()
    }
}
