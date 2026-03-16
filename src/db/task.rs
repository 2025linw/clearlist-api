use chrono::NaiveDate;
use sqlx::{PgPool, QueryBuilder};
use uuid::Uuid;

use super::Result;
use crate::{
    com::model::{Task, db::SQLCmp},
    db::query_as_wrapper,
};

pub async fn query_tasks(
    conn: &PgPool,
    user_id: Uuid,
    limit: i64,
    offset: i64,
    start_filter: Option<Vec<(SQLCmp, NaiveDate)>>,
    deadline_filter: Option<Vec<(SQLCmp, NaiveDate)>>,
) -> Result<Vec<Task>> {
    let mut builder = QueryBuilder::new("SELECT * FROM app.tasks WHERE created_by = ");
    builder.push_bind(&user_id);

    if let Some(start) = start_filter {
        // TODO: update this to match with start_date or start_at
        for (cmp, date) in start {
            builder.push(" AND ");

            builder.push(format!("(start_date {} ", cmp));
            builder.push_bind(date);
            builder.push(format!(") OR (start_at {} ", cmp));
            builder.push_bind(date);
            builder.push(")");
        }
    }
    if let Some(deadline) = deadline_filter {
        for (cmp, date) in deadline {
            builder.push(" AND ");

            builder.push(format!("deadline {} ", cmp));
            builder.push_bind(date);
        }
    }
    builder.push(" LIMIT ");
    builder.push_bind(limit);
    builder.push(" OFFSET ");
    builder.push_bind(offset);

    let query = builder.build_query_as::<Task>();

    let mut tasks = query.fetch_all(conn).await?;

    // TODO: do this in one query then map?
    for task in tasks.iter_mut() {
        task.tags = Some(tag::query_task_tags(conn, task.id, user_id.clone()).await?);
    }

    Ok(tasks)
}

pub async fn insert_task(conn: &PgPool, user_id: Uuid, task: Task) -> Result<Uuid> {
    let mut transaction = conn.begin().await?;

    let task_id = sqlx::query_scalar(
        "INSERT INTO app.tasks (title, notes, start_date, start_at, deadline, created_by)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING id",
    )
    .bind(task.title)
    .bind(task.notes)
    .bind(task.start_date)
    .bind(task.start_at)
    .bind(task.deadline)
    .bind(&user_id)
    .fetch_one(&mut *transaction)
    .await?;

    if let Some(tags) = task.tags {
        tag::update_task_tags(
            &mut *transaction,
            task_id,
            tags.iter().map(|tag| tag.id).collect(),
        )
        .await?;
    }

    transaction.commit().await?;

    Ok(task_id)
}

pub async fn select_task(conn: &PgPool, task_id: Uuid, user_id: Uuid) -> Result<Option<Task>> {
    let task_opt = query_as_wrapper::<Task>(
        "SELECT *
        FROM app.tasks
        WHERE id = $1 AND created_by = $2",
    )
    .bind(task_id)
    .bind(&user_id)
    .fetch_optional(conn)
    .await?;

    match task_opt {
        None => Ok(None),
        Some(mut task) => {
            task.tags = Some(tag::query_task_tags(conn, task_id, user_id).await?);

            Ok(Some(task))
        }
    }
}

pub async fn update_task(
    conn: &PgPool,
    task_id: Uuid,
    user_id: Uuid,
    task: Task,
) -> Result<Option<Task>> {
    let mut transaction = conn.begin().await?;

    if sqlx::query(
        "UPDATE app.tasks SET
        (updated_at, title, notes, start_date, start_at, deadline) =
        (CURRENT_TIMESTAMP, $3, $4, $5, $6, $7)
        WHERE id = $1 AND created_by = $2",
    )
    .bind(task_id)
    .bind(&user_id)
    .bind(task.title)
    .bind(task.notes)
    .bind(task.start_date)
    .bind(task.start_at)
    .bind(task.deadline)
    .execute(&mut *transaction)
    .await?
    .rows_affected()
        == 0
    {
        return Ok(None);
    }

    if let Some(tags) = task.tags {
        tag::update_task_tags(
            &mut *transaction,
            task_id,
            tags.iter().map(|tag| tag.id).collect(),
        )
        .await?;
    }

    transaction.commit().await?;

    select_task(conn, task_id, user_id).await
}

pub async fn delete_task(conn: &PgPool, task_id: Uuid, user_id: Uuid) -> Result<Option<()>> {
    let mut transaction = conn.begin().await?;

    if sqlx::query("DELETE FROM app.tasks WHERE id = $1 AND created_by = $2")
        .bind(task_id)
        .bind(user_id)
        .execute(&mut *transaction)
        .await?
        .rows_affected()
        == 0
    {
        return Ok(None);
    }

    transaction.commit().await?;

    Ok(Some(()))
}

pub mod tag {
    use sqlx::{Executor, Postgres, QueryBuilder};
    use uuid::Uuid;

    use crate::{
        com::model::Tag,
        db::{Result, query_as_wrapper},
    };

    // NOTE: all these functions requires/expect that the task with task_id exists

    pub async fn query_task_tags<'e, E>(conn: E, task_id: Uuid, user_id: Uuid) -> Result<Vec<Tag>>
    where
        E: Executor<'e, Database = Postgres>,
    {
        Ok(query_as_wrapper::<Tag>(
            "SELECT *
            FROM app.tags tg
            JOIN app.task_tags tt ON tg.id = tt.tag_id
            JOIN app.tasks t ON tt.task_id = t.id
            WHERE tt.task_id = $1 AND t.created_by = $2",
        )
        .bind(task_id)
        .bind(user_id)
        .fetch_all(conn)
        .await?)
    }

    pub async fn update_task_tags<'e, E>(
        conn: E,
        task_id: Uuid,
        tag_ids: Vec<Uuid>,
    ) -> Result<Option<()>>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let mut builder =
            QueryBuilder::new("WITH deleted AS (DELETE FROM app.task_tags WHERE task_id = ");
        builder.push_bind(task_id);
        builder.push(" RETURNING task_id) INSERT INTO app.task_tags (task_id, tag_id) SELECT COALESCE(deleted.task_id, ");
        builder.push_bind(task_id);
        builder.push("), unnest_tag FROM deleted RIGHT JOIN UNNEST(");
        builder.push_bind(&tag_ids);
        builder.push(") AS unnest_tag ON TRUE");

        if builder.build().execute(conn).await?.rows_affected() != tag_ids.len() as u64 {
            return Ok(None);
        }

        Ok(Some(()))
    }

    pub async fn add_task_tag<'e, E>(conn: E, task_id: Uuid, tag_id: Uuid) -> Result<Option<()>>
    where
        E: Executor<'e, Database = Postgres>,
    {
        if sqlx::query("INSERT INTO app.task_tags (task_id, tag_id) VALUES ($1, $2)")
            .bind(task_id)
            .bind(tag_id)
            .execute(conn)
            .await?
            .rows_affected()
            != 1
        {
            return Ok(None);
        }

        Ok(Some(()))
    }

    pub async fn delete_task_tag<'e, E>(conn: E, task_id: Uuid, tag_id: Uuid) -> Result<Option<()>>
    where
        E: Executor<'e, Database = Postgres>,
    {
        if sqlx::query("DELETE FROM app.task_tags WHERE task_id = $1 AND tag_id = $2")
            .bind(task_id)
            .bind(tag_id)
            .execute(conn)
            .await?
            .rows_affected()
            == 0
        {
            return Ok(None);
        }

        Ok(Some(()))
    }
}
