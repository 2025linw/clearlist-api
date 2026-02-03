use uuid::Uuid;

use crate::error::Result;

use super::DatabaseConn;

impl DatabaseConn {
    pub async fn query_area() -> Result<()> {
        todo!()
    }

    pub async fn create_area() -> Result<()> {
        todo!()
    }

    pub async fn retrieve_area() -> Result<()> {
        todo!()
    }

    pub async fn update_area() -> Result<()> {
        todo!()
    }

    pub async fn delete_area(area_id: Uuid, user_id: Uuid) -> Result<()> {
        todo!()
    }
}
