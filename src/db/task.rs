use deadpool_postgres::Object;
use tokio_postgres::Row;
use uuid::Uuid;

use crate::com::{
    model::{Tag, Task, query::DateFilter},
    util::{SQLBuilder, SQLCmp},
};

use super::{Error, Result};

const RETURNING: [&str; 6] = [
    "id",
    "title",
    "notes",
    "start_date",
    "start_time",
    "deadline",
];

pub async fn query_tasks(
    conn: &Object,
    limit: i64,
    offset: i64,
    start_filter: Option<DateFilter>,
    deadline_filter: Option<DateFilter>,
) -> Result<Vec<Task>> {
    let mut builder = SQLBuilder::new("clear_list.tasks");

    builder.set_returning_str(&RETURNING);

    builder.set_limit(limit);
    builder.set_offset(offset);

    if let Some(start) = start_filter {
        match start {
            DateFilter::Exact(date) => {
                builder.add_condition("start_date", SQLCmp::Equal, date);
            }
            DateFilter::Range([start, end]) => {
                builder.add_condition("start_date", SQLCmp::GreaterThanEqual, start);
                builder.add_condition("start_date", SQLCmp::LessThanEqual, end);
            }
        }
    }
    if let Some(deadline) = deadline_filter {
        match deadline {
            DateFilter::Exact(date) => {
                builder.add_condition("deadline", SQLCmp::Equal, date);
            }
            DateFilter::Range([start, end]) => {
                builder.add_condition("deadline", SQLCmp::GreaterThanEqual, start);
                builder.add_condition("deadline", SQLCmp::LessThanEqual, end);
            }
        }
    }

    let rows = conn
        .query(&builder.select_query(), &builder.params())
        .await?;

    let mut tasks: Vec<Task> = rows.into_iter().map(|row| row.into()).collect();

    for task in tasks.iter_mut() {
        task.tags = get_task_tags(conn, task.id).await?;
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

    for tag in task.tags {
        if transaction
            .execute(
                "INSERT INTO clear_list.task_tags (task_id, tag_id) VALUES ($1, $2);",
                &[&task_id, &tag.id],
            )
            .await?
            != 1
        {
            return Err(Error::DatabaseOp(format!(
                "failed to insert tag {} when adding task",
                tag.id
            )));
        }
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
            task.tags = get_task_tags(conn, task_id).await?;

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
    for tag in task.tags {
        if transaction.execute("", &[]).await? != 1 {
            return Err(Error::DatabaseOp(format!(
                "failed to insert tag {} when updating task",
                tag.id
            )));
        }
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

pub async fn get_task_tags(conn: &Object, task_id: Uuid) -> Result<Vec<Tag>> {
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
