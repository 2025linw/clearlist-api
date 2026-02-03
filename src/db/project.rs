use uuid::Uuid;

use crate::error::Result;

use super::DatabaseConn;

impl DatabaseConn {
    pub async fn query_project() -> Result<()> {
        todo!()
    }

    pub async fn create_project() -> Result<()> {
        todo!()
    }

    pub async fn retrieve_project() -> Result<()> {
        todo!()
    }

    pub async fn update_project() -> Result<()> {
        todo!()
    }

    pub async fn delete_project(project_id: Uuid, user_id: Uuid) -> Result<()> {
        todo!()
    }
}
