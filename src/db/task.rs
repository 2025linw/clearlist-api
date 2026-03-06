use chrono::NaiveDate;
use sqlx::{PgPool, Postgres, QueryBuilder, query, query_scalar};
use uuid::Uuid;

use super::Result;
use crate::com::model::{Task, db::SQLCmp};

pub async fn query_tasks(
    conn: &PgPool,
    limit: i64,
    offset: i64,
    start_filter: Option<Vec<(SQLCmp, NaiveDate)>>,
    deadline_filter: Option<Vec<(SQLCmp, NaiveDate)>>,
) -> Result<Vec<Task>> {
    let mut builder = QueryBuilder::new("SELECT * FROM clear_list.tasks");

    let mut has_where = false;

    if let Some(start) = start_filter {
        for (cmp, date) in start {
            if !has_where {
                builder.push(" WHERE ");

                has_where = true;
            } else {
                builder.push(" AND ");
            }

            builder.push(format!("start_date {} ", cmp));
            builder.push_bind(date);
        }
    }
    if let Some(deadline) = deadline_filter {
        for (cmp, date) in deadline {
            if !has_where {
                builder.push(" WHERE ");

                has_where = true;
            } else {
                builder.push(" AND ");
            }

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

    // TODO: do this in main query?
    for task in tasks.iter_mut() {
        task.tags = Some(tag::query_task_tags(conn, task.id).await?);
    }

    Ok(tasks)
}

pub async fn insert_task(conn: &PgPool, task: Task) -> Result<Uuid> {
    let mut transaction = conn.begin().await?;

    let task_id = query_scalar!(
        "INSERT INTO clear_list.tasks (title, notes, start_date, start_time, deadline)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id",
        task.title,
        task.notes,
        task.start_date,
        task.start_time,
        task.deadline,
    )
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

pub async fn select_task(conn: &PgPool, task_id: Uuid) -> Result<Option<Task>> {
    let task_opt = sqlx::query_as::<Postgres, Task>(
        "SELECT id, title, notes, start_date, start_time, deadline
        FROM clear_list.tasks
        WHERE id = $1;",
    )
    .bind(task_id)
    .fetch_optional(conn)
    .await?;

    match task_opt {
        None => Ok(None),
        Some(mut task) => {
            task.tags = Some(tag::query_task_tags(conn, task_id).await?);

            Ok(Some(task))
        }
    }
}

pub async fn update_task(conn: &PgPool, task_id: Uuid, task: Task) -> Result<Option<Task>> {
    let mut transaction = conn.begin().await?;

    if query!(
        "UPDATE clear_list.tasks SET
        (title, notes, start_date, start_time, deadline) =
        ($2, $3, $4, $5, $6)
        WHERE id = $1;",
        task_id,
        task.title,
        task.notes,
        task.start_date,
        task.start_time,
        task.deadline,
    )
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

    select_task(conn, task_id).await
}

pub async fn delete_task(conn: &PgPool, task_id: Uuid) -> Result<Option<()>> {
    let mut transaction = conn.begin().await?;

    if query!("DELETE FROM clear_list.tasks WHERE id = $1;", task_id)
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
    use sqlx::{Executor, Postgres, QueryBuilder, query, query_as};
    use uuid::Uuid;

    use crate::{com::model::Tag, db::Result};

    pub async fn query_task_tags<'e, E>(conn: E, task_id: Uuid) -> Result<Vec<Tag>>
    where
        E: Executor<'e, Database = Postgres>,
    {
        Ok(query_as!(
            Tag,
            "SELECT tg.id, tg.label, tg.category
            FROM clear_list.tags tg
            JOIN clear_list.task_tags tt ON tg.id = tt.tag_id
            WHERE tt.task_id = $1",
            task_id
        )
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
            QueryBuilder::new("WITH deleted AS (DELETE FROM clear_list.task_tags WHERE task_id = ");
        builder.push_bind(task_id);

        builder.push(" RETURNING *), deleted_task_id AS (SELECT DISTINCT task_id FROM deleted) INSERT INTO clear_list.task_tags (task_id, tag_id) SELECT * FROM deleted_task_id CROSS JOIN UNNEST(");
        builder.push_bind(tag_ids);
        builder.push(")");

        builder.build().execute(conn).await?;

        Ok(Some(()))
    }

    pub async fn add_task_tag<'e, E>(conn: E, task_id: Uuid, tag_id: Uuid) -> Result<Option<()>>
    where
        E: Executor<'e, Database = Postgres>,
    {
        if query!(
            "INSERT INTO clear_list.task_tags (task_id, tag_id) VALUES ($1, $2)",
            task_id,
            tag_id
        )
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
        if query!(
            "DELETE FROM clear_list.task_tags WHERE task_id = $1 AND tag_id = $2",
            task_id,
            tag_id
        )
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
