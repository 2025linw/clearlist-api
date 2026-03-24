use std::collections::{HashMap, hash_map::Entry};

use chrono::NaiveDate;
use sqlx::{PgPool, Postgres, QueryBuilder, Transaction};
use uuid::Uuid;

use super::{Error, Result};
use crate::{
    com::model::{Tag, Task, TaskTag, db::SQLCmp},
    db::query_as_wrapper,
};

pub async fn query_tasks(
    conn: PgPool,
    user_id: Uuid,
    limit: i64,
    offset: i64,
    deleted: bool,
    start_filter: Option<Vec<(SQLCmp, NaiveDate)>>,
    deadline_filter: Option<Vec<(SQLCmp, NaiveDate)>>,
) -> Result<Vec<Task>> {
    let mut builder = QueryBuilder::new("SELECT * FROM app.tasks WHERE created_by = ");
    builder.push_bind(user_id);
    if deleted {
        builder.push(" AND deleted_at IS NOT NULL");
    } else {
        builder.push(" AND deleted_at IS NULL");
    }
    if let Some(start) = start_filter {
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
    builder.push(" GROUP BY t.id");
    builder.push(" ORDER BY updated_at DESC");
    builder.push(format!(" LIMIT {}", limit));
    builder.push(format!(" OFFSET {}", offset));

    let query = builder.build_query_as::<Task>();

    let mut tasks = query.fetch_all(&conn).await?;

    let task_ids: Vec<Uuid> = tasks.iter().map(|task| task.id).collect();
    let tags = query_as_wrapper::<TaskTag>(
        "SELECT tt.task_id, tg.*
            FROM app.task_tags tt
            LEFT JOIN app.tags tg ON tt.tag_id = tg.id
            WHERE tt.task_id = ANY($1)",
    )
    .bind(task_ids)
    .fetch_all(&conn)
    .await?;

    let mut task_tag_map: HashMap<Uuid, Vec<Tag>> = HashMap::new();
    for TaskTag { task_id, tag } in tags {
        if let Entry::Vacant(e) = task_tag_map.entry(task_id) {
            e.insert(vec![tag]);
        } else {
            task_tag_map.get_mut(&task_id).unwrap().push(tag);
        }
    }
    for task in tasks.iter_mut() {
        if task_tag_map.contains_key(&task.id) {
            task.tags = task_tag_map.remove(&task.id).unwrap();
        }
    }

    Ok(tasks)
}

pub async fn insert_task(conn: PgPool, user_id: Uuid, task: Task) -> Result<Uuid> {
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
    .bind(user_id)
    .fetch_one(&mut *transaction)
    .await?;

    if !task.tags.is_empty() {
        update_tag_helper(&mut transaction, task_id, task.tags).await?;
    }

    transaction.commit().await?;

    Ok(task_id)
}

pub async fn select_task(conn: PgPool, task_id: Uuid, user_id: Uuid) -> Result<Option<Task>> {
    let task_opt = query_as_wrapper::<Task>(
        "SELECT *
        FROM app.tasks
        WHERE id = $1 AND created_by = $2 AND deleted_at IS NULL",
    )
    .bind(task_id)
    .bind(user_id)
    .fetch_optional(&conn)
    .await?;

    match task_opt {
        None => Ok(None),
        Some(mut task) => {
            let tags = query_as_wrapper::<Tag>(
                "SELECT tg.*
                FROM app.task_tags tt
                JOIN app.tags tg ON tt.tag_id = tg.id
                WHERE tt.task_id = $1 AND tg.deleted_at IS NULL",
            )
            .bind(task_id)
            .fetch_all(&conn)
            .await?;

            task.tags = tags;

            Ok(Some(task))
        }
    }
}

pub async fn update_task(
    conn: PgPool,
    task_id: Uuid,
    user_id: Uuid,
    task: Task,
) -> Result<Option<Task>> {
    let mut transaction = conn.begin().await?;

    if sqlx::query(
        "UPDATE app.tasks SET
        (updated_at, title, notes, start_date, start_at, deadline) =
        (CURRENT_TIMESTAMP, $3, $4, $5, $6, $7)
        WHERE id = $1 AND created_by = $2 AND deleted_at IS NULL",
    )
    .bind(task_id)
    .bind(user_id)
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

    if !task.tags.is_empty() {
        update_tag_helper(&mut transaction, task_id, task.tags).await?;
    }

    transaction.commit().await?;

    select_task(conn, task_id, user_id).await
}

pub async fn delete_task(conn: PgPool, task_id: Uuid, user_id: Uuid) -> Result<Option<()>> {
    let mut transaction = conn.begin().await?;

    if sqlx::query(
        "UPDATE app.tasks SET
        (updated_at, deleted_at) =
        (CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
        WHERE id = $1 AND created_by = $2 AND deleted_at IS NULL",
    )
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

pub async fn update_tag_helper(
    tx: &mut Transaction<'_, Postgres>,
    task_id: Uuid,
    tags: Vec<Tag>,
) -> Result<()> {
    let mut builder = QueryBuilder::new("INSERT INTO app.task_tags (task_id, tag_id) VALUES");

    let mut separated = builder.separated(", ");
    for tag in tags.iter() {
        separated.push(" (");
        separated.push_bind(task_id);
        separated.push(", ");
        separated.push_bind(tag.id);
        separated.push(")");
    }

    let num_rows = builder.build().execute(&mut **tx).await?.rows_affected() as usize;

    if num_rows != tags.len() {
        return Err(Error::Operation(format!(
            "expected: {} tags; got: {}",
            num_rows,
            tags.len()
        )));
    }

    Ok(())
}

pub mod tag {
    use sqlx::{PgPool, QueryBuilder};
    use uuid::Uuid;

    use crate::{com::model::Tag, db::query_as_wrapper};

    use super::Result;

    pub async fn query_task_tags(conn: PgPool, task_id: Uuid, user_id: Uuid) -> Result<Vec<Tag>> {
        Ok(query_as_wrapper::<Tag>(
            "SELECT *
                FROM app.tags tg
                JOIN app.task_tags tt ON tg.id = tt.tag_id
                JOIN app.tasks t ON tt.task_id = t.id AND t.deleted_at IS NULL
                WHERE tt.task_id = $1 AND t.created_by = $2 AND tg.deleted_at IS NULL",
        )
        .bind(task_id)
        .bind(user_id)
        .fetch_all(&conn)
        .await?)
    }

    pub async fn update_task_tags(
        conn: PgPool,
        task_id: Uuid,
        tag_ids: Vec<Uuid>,
    ) -> Result<Option<()>> {
        let mut builder =
            QueryBuilder::new("WITH deleted AS (DELETE FROM app.task_tags WHERE task_id = ");
        builder.push_bind(task_id);
        builder.push(" RETURNING task_id) INSERT INTO app.task_tags (task_id, tag_id) SELECT COALESCE(deleted.task_id, ");
        builder.push_bind(task_id);
        builder.push("), unnest_tag FROM deleted RIGHT JOIN UNNEST(");
        builder.push_bind(&tag_ids);
        builder.push(") AS unnest_tag ON TRUE");

        if builder.build().execute(&conn).await?.rows_affected() != tag_ids.len() as u64 {
            return Ok(None);
        }

        Ok(Some(()))
    }

    pub async fn add_task_tag(conn: PgPool, task_id: Uuid, tag_id: Uuid) -> Result<Option<()>> {
        sqlx::query("INSERT INTO app.task_tags (task_id, tag_id) VALUES ($1, $2)")
            .bind(task_id)
            .bind(tag_id)
            .execute(&conn)
            .await?;

        Ok(Some(()))
    }

    pub async fn delete_task_tag(conn: PgPool, task_id: Uuid, tag_id: Uuid) -> Result<Option<()>> {
        if sqlx::query("DELETE FROM app.task_tags WHERE task_id = $1 AND tag_id = $2")
            .bind(task_id)
            .bind(tag_id)
            .execute(&conn)
            .await?
            .rows_affected()
            == 0
        {
            return Ok(None);
        }

        Ok(Some(()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
}
