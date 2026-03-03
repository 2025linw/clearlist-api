use deadpool_postgres::Object;
use tokio_postgres::Row;
use uuid::Uuid;

use crate::com::model::Task;

use super::Result;

pub async fn query_tasks(conn: &Object, limit: i64, offset: i64) -> Result<Vec<Task>> {
    let rows: Vec<Row> = conn
        .query(
            "SELECT id, title, notes, start_date, start_time, deadline
            FROM clear_list.tasks
            LIMIT $1 OFFSET $2;",
            &[&limit, &offset],
        )
        .await?;

    let mut tasks: Vec<Task> = rows.into_iter().map(|row| row.into()).collect();

    for task in tasks.iter_mut() {
        task.tags = Some(tag::query_task_tags(conn, task.id).await?);
    }

    Ok(tasks)
}

pub async fn insert_task(conn: &mut Object, task: Task) -> Result<Uuid> {
    let transaction = conn.transaction().await?;

    let row: Row = transaction
        .query_one(
            "INSERT INTO clear_list.tasks (title, notes, start_date, start_time, deadline)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id;",
            &[
                &task.title,
                &task.notes,
                &task.start_date,
                &task.start_time,
                &task.deadline,
            ],
        )
        .await?;
    let task_id: Uuid = row.get("id");

    if let Some(tags) = task.tags {
        tag::update_task_tags(
            &transaction,
            task_id,
            tags.iter().map(|tag| tag.id).collect(),
        )
        .await?;
    }

    transaction.commit().await?;

    Ok(task_id)
}

pub async fn select_task(conn: &Object, task_id: Uuid) -> Result<Option<Task>> {
    let row_opt: Option<Row> = conn
        .query_opt(
            "SELECT id, title, notes, start_date, start_time, deadline
            FROM clear_list.tasks
            WHERE id = $1;",
            &[&task_id],
        )
        .await?;

    match row_opt {
        None => Ok(None),
        Some(row) => {
            let mut task: Task = row.into();
            task.tags = Some(tag::query_task_tags(conn, task_id).await?);

            Ok(Some(task))
        }
    }
}

pub async fn update_task(conn: &mut Object, task_id: Uuid, task: Task) -> Result<Option<Task>> {
    let transaction = conn.transaction().await?;

    if transaction
        .execute(
            "UPDATE clear_list.tasks SET
            (title, notes, start_date, start_time, deadline) =
            ($2, $3, $4, $5, $6)
            WHERE id = $1;",
            &[
                &task_id,
                &task.title,
                &task.notes,
                &task.start_date,
                &task.start_time,
                &task.deadline,
            ],
        )
        .await?
        != 1
    {
        return Ok(None);
    }

    transaction
        .execute(
            "DELETE FROM clear_list.task_tags WHERE task_id = $1;",
            &[&task_id],
        )
        .await?;

    if let Some(tags) = task.tags {
        tag::update_task_tags(
            &transaction,
            task_id,
            tags.iter().map(|tag| tag.id).collect(),
        )
        .await?;
    }

    transaction.commit().await?;

    select_task(conn, task_id).await
}

pub async fn delete_task(conn: &mut Object, task_id: Uuid) -> Result<Option<()>> {
    let transaction = conn.transaction().await?;

    if transaction
        .execute("DELETE FROM clear_list.tasks WHERE id = $1;", &[&task_id])
        .await?
        != 1
    {
        return Ok(None);
    }

    transaction.commit().await?;

    Ok(Some(()))
}

pub mod tag {
    use deadpool_postgres::{Object, Transaction};
    use uuid::Uuid;

    use crate::{
        com::model::Tag,
        db::{Error, Result},
    };

    pub async fn query_task_tags(conn: &Object, task_id: Uuid) -> Result<Vec<Tag>> {
        let rows = conn
            .query(
                "SELECT tg.id, tg.label, tg.category
            FROM clear_list.tags tg
            JOIN clear_list.task_tags tt ON tg.id = tt.tag_id
            WHERE tt.task_id = $1;",
                &[&task_id],
            )
            .await?;

        Ok(rows.into_iter().map(|row| row.into()).collect())
    }

    pub async fn update_task_tags(
        transaction: &Transaction<'_>,
        task_id: Uuid,
        tag_ids: Vec<Uuid>,
    ) -> Result<Option<()>> {
        transaction
            .execute(
                "DELETE FROM clear_list.task_tags WHERE task_id = $1;",
                &[&task_id],
            )
            .await?;

        for tag_id in tag_ids {
            if transaction
                .execute(
                    "INSERT INTO clear_list.task_tags (task_id, tag_id) VALUES ($1, $2);",
                    &[&task_id, &tag_id],
                )
                .await?
                != 1
            {
                return Err(Error::DatabaseOp(format!(
                    "failed to insert tag {} for task {}",
                    tag_id, task_id,
                )));
            }
        }

        Ok(Some(()))
    }

    pub async fn update_task_tags_query(
        conn: &mut Object,
        task_id: Uuid,
        tag_ids: Vec<Uuid>,
    ) -> Result<Option<()>> {
        let transaction = conn.transaction().await?;

        let res = update_task_tags(&transaction, task_id, tag_ids).await?;

        transaction.commit().await?;

        Ok(res)
    }

    pub async fn add_task_tag(
        transaction: &Transaction<'_>,
        task_id: Uuid,
        tag_id: Uuid,
    ) -> Result<Option<()>> {
        if transaction
            .execute(
                "INSERT INTO clear_list.task_tags (task_id, tag_id) VALUES ($1, $2);",
                &[&task_id, &tag_id],
            )
            .await?
            != 1
        {
            return Ok(None);
        }

        Ok(Some(()))
    }

    pub async fn add_task_tag_query(
        conn: &mut Object,
        task_id: Uuid,
        tag_id: Uuid,
    ) -> Result<Option<()>> {
        let transaction = conn.transaction().await?;

        let res = add_task_tag(&transaction, task_id, tag_id).await?;

        transaction.commit().await?;

        Ok(res)
    }

    pub async fn delete_task_tag(
        transaction: &Transaction<'_>,
        task_id: Uuid,
        tag_id: Uuid,
    ) -> Result<Option<()>> {
        if transaction
            .execute(
                "DELETE FROM clear_list.task_tags WHERE task_id = $1 AND tag_id = $2",
                &[&task_id, &tag_id],
            )
            .await?
            != 1
        {
            return Ok(None);
        }

        Ok(Some(()))
    }

    pub async fn delete_task_tag_query(
        conn: &mut Object,
        task_id: Uuid,
        tag_id: Uuid,
    ) -> Result<Option<()>> {
        let transaction = conn.transaction().await?;

        let res = delete_task_tag(&transaction, task_id, tag_id).await?;

        transaction.commit().await?;

        Ok(res)
    }
}
