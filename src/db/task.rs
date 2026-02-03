use chrono::{NaiveDate, NaiveTime};
use tokio_postgres::types::ToSql;
use uuid::Uuid;

use crate::{error::Result, models::task::DatabaseModel};

use super::{
    DatabaseConn,
    utils::{DateCmp, DateTimeFilter},
};

const OUTPUT: &str = "";

impl DatabaseConn {
    pub async fn query_task(
        &self,
        user_id: Uuid,

        limit: usize,
        offset: usize,

        search: Option<String>,

        start_date: Option<DateCmp<NaiveDate>>,
        start_time: Option<DateCmp<NaiveTime>>,
        deadline: Option<DateCmp<NaiveDate>>,

        project_id: Option<Uuid>,
        area_id: Option<Uuid>,

        completed: Option<bool>,
        logged: Option<bool>,
        deleted: Option<bool>,
    ) -> Result<Vec<DatabaseModel>> {
        let mut query = vec![format!("SELECT {OUTPUT} FROM clear_list.task")];
        let mut params: Vec<&(dyn ToSql + Sync)> = vec![];

        let search_query: String;
        if search.is_some() {
            query.push(format!("{} = ", DatabaseModel::TITLE));
            search_query = format!("%${}%", params.len() + 1);

            params.push(&search_query);
        }
        if let Some(cmp) = start_date {
            query.push(cmp.to_sql(DatabaseModel::START_DATE, params.len() + 1));

            params.push(&cmp)
        }
        if let Some(cmp) = start_time {
            query.push(cmp.to_sql(DatabaseModel::START_DATE, params.len() + 1));
        }
        if let Some(cmp) = deadline {
            query.push(cmp.to_sql(DatabaseModel::START_DATE, params.len() + 1));
        }

        query.push(format!("LIMIT {limit}"));
        query.push(format!("OFFSET {offset}"));

        let query_str = query.join(" ");

        println!("{query_str}");

        let tasks = self
            .get_conn()
            .await?
            .query(&query_str, &params)
            .await?
            .iter()
            .map(|row| DatabaseModel::from(row))
            .collect();

        Ok(tasks)
    }

    pub async fn create_task(
        &self,
        user_id: Uuid,

        title: Option<String>,
        notes: Option<String>,

        start_date: Option<NaiveDate>,
        start_time: Option<NaiveTime>,
        deadline: Option<NaiveDate>,

        project_id: Option<Uuid>,
        area_id: Option<Uuid>,
    ) -> Result<DatabaseModel> {
        todo!()
    }

    pub async fn retrieve_task(&self, task_id: Uuid, user_id: Uuid) -> Result<DatabaseModel> {
        todo!()
    }

    pub async fn update_task(
        &self,

        task_id: Uuid,
        user_id: Uuid,

        title: Option<String>,
        notes: Option<String>,
        start_date: Option<NaiveDate>,
        start_time: Option<NaiveTime>,
        deadline: Option<NaiveDate>,
        completed: Option<bool>,
        logged: Option<bool>,
        project_id: Option<Uuid>,
        area_id: Option<Uuid>,
        deleted: Option<bool>,
    ) -> Result<DatabaseModel> {
        todo!()
    }

    pub async fn delete_task(task_id: Uuid, user_id: Uuid) -> Result<DatabaseModel> {
        todo!()
    }
}
