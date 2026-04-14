//! # Database Task Module
//!
//! This module contains collection of database functions for tasks

pub mod tag;

use std::collections::{HashMap, hash_map::Entry};

use sqlx::{PgConnection, PgPool, QueryBuilder};
use uuid::Uuid;

use super::{
    Error, Result,
    error::ApplicationError,
    filters::{DateFilter, SQLCmp, TaskSort},
    query_as_wrapper,
    task::tag::update_task_tags_inner,
};
use crate::{
    com::constants::{DEFAULT_LIMIT, MAX_LIMIT},
    models::{Tag, Task, TaskTag},
    routes::models::{SortOrder, task::Model as TaskCreate},
};

/// Options for querying tasks in database
///
/// This contains filters for querying tasks:
///
/// * `limit`: limits number of tasks to return (default: 50)
/// * `offset`: number of tasks to skip (default: 0)
/// * `sort_order`: order to return tasks (default: recently updated first (updated decreasing))
/// * `completed`: filter by completion status (default: false)
/// * `deleted`: filter by deletion status (default: false)
/// * `start_filter`: filter by start date range
/// * `deadline_filter`: filter by deadline range
#[derive(Default)]
pub struct TaskQueryOptions {
    pub limit: Option<i64>,
    pub offset: Option<i64>,

    pub sort_order: TaskSort,

    pub completed: bool,
    pub deleted: bool,

    pub start_filter: Option<DateFilter>,
    pub deadline_filter: Option<DateFilter>,
}

/// Query database for tasks
///
/// # Arguments
///
/// * `pool`: Database connection pool
/// * `user_id`: User ID to query tasks for
/// * `opts`: Query filter
///
/// # Returns
///
/// List of tasks
pub async fn query_tasks(
    pool: PgPool,
    user_id: Uuid,
    opts: Option<TaskQueryOptions>,
) -> Result<Vec<Task>> {
    let mut conn = pool.acquire().await?;
    let tasks = query_tasks_inner(&mut conn, user_id, opts.unwrap_or_default()).await?;
    conn.close().await?;

    Ok(tasks)
}

/// Internal function for `query_tasks`
///
/// Only used internally
async fn query_tasks_inner(
    conn: &mut PgConnection,
    user_id: Uuid,
    opts: TaskQueryOptions,
) -> Result<Vec<Task>> {
    let limit = opts.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
    let offset = opts.offset.unwrap_or(0).max(0);

    let mut builder = QueryBuilder::new("SELECT * FROM app.tasks WHERE created_by = ");
    builder.push_bind(user_id);
    if opts.completed {
        builder.push(" AND completed_at IS NOT NULL");
    } else {
        builder.push(" AND completed_at IS NULL");
    }
    if opts.deleted {
        builder.push(" AND deleted_at IS NOT NULL");
    } else {
        builder.push(" AND deleted_at IS NULL");
    }
    if let Some(start) = opts.start_filter {
        builder.push(" AND ");

        let mut separated = builder.separated(" AND ");
        for (cmp, date) in start.into_sql() {
            if matches!(cmp, SQLCmp::Exists | SQLCmp::NotExists) {
                separated.push(format!("start_dt {}", cmp));
            } else {
                separated.push(format!("((start_dt::date {} ", cmp));
                separated.push_bind_unseparated(date);
                separated.push_unseparated(")");

                if matches!(cmp, SQLCmp::NotEqual) {
                    separated.push_unseparated(" OR (start_dt IS NULL)");
                }
                separated.push_unseparated(")");
            }
        }
    }
    if let Some(deadline) = opts.deadline_filter {
        builder.push(" AND ");

        let mut separated = builder.separated(" AND ");
        for (cmp, date) in deadline.into_sql() {
            if matches!(cmp, SQLCmp::Exists | SQLCmp::NotExists) {
                separated.push(format!("(deadline {})", cmp));
            } else {
                separated.push(format!("((deadline {} ", cmp));
                separated.push_bind_unseparated(date);
                separated.push_unseparated(")");

                if matches!(cmp, SQLCmp::NotEqual) {
                    separated.push_unseparated(" OR (deadline IS NULL)");
                }
                separated.push_unseparated(")");
            }
        }
    }
    builder.push(" GROUP BY id");
    match opts.sort_order {
        TaskSort::Updated(SortOrder::Descending) => builder.push(" ORDER BY updated_at DESC"),
        TaskSort::Updated(SortOrder::Ascending) => builder.push(" ORDER BY updated_at ASC"),
        TaskSort::Created(SortOrder::Descending) => builder.push(" ORDER BY created_at DESC"),
        TaskSort::Created(SortOrder::Ascending) => builder.push(" ORDER BY created_at ASC"),
        TaskSort::Title(SortOrder::Ascending) => builder.push(" ORDER BY title ASC"),
        TaskSort::Title(SortOrder::Descending) => builder.push(" ORDER BY title DESC"),
        TaskSort::Start(SortOrder::Ascending) => builder.push(" ORDER BY start_dt ASC NULLS LAST"),
        TaskSort::Start(SortOrder::Descending) => {
            builder.push(" ORDER BY start_dt DESC NULLS LAST")
        }
        TaskSort::Deadline(SortOrder::Ascending) => {
            builder.push(" ORDER BY deadline ASC NULLS LAST")
        }
        TaskSort::Deadline(SortOrder::Descending) => {
            builder.push(" ORDER BY deadline DESC NULLS LAST")
        }
    };
    builder.push(" LIMIT ");
    builder.push_bind(limit);
    builder.push(" OFFSET ");
    builder.push_bind(offset);

    let query = builder.build_query_as::<Task>();

    let mut tasks = query.fetch_all(conn.as_mut()).await?;

    // Get tags
    let task_ids: Vec<Uuid> = tasks.iter().map(|task| task.id).collect();
    let tags = query_as_wrapper::<TaskTag>(
        "SELECT tt.task_id, tg.*
            FROM app.task_tags tt
            LEFT JOIN app.tags tg ON tt.tag_id = tg.id
            WHERE tt.task_id = ANY($1)",
    )
    .bind(task_ids)
    .fetch_all(conn.as_mut())
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

/// Select task from database
///
/// # Arguments
///
/// * `pool`: Database connection pool
/// * `task_id`: ID of task being retrieved
/// * `user_id`: User ID of task owner
///
/// # Returns
///
/// Task wrapped in `Some`, if exists
///
/// `None`, if it does not exist
pub async fn select_task(pool: PgPool, task_id: Uuid, user_id: Uuid) -> Result<Option<Task>> {
    let mut conn = pool.acquire().await?;
    let task_opt = select_task_inner(&mut conn, task_id, user_id).await?;
    conn.close().await?;

    Ok(task_opt)
}

/// Internal function for `select_task`
///
/// Only used internally
async fn select_task_inner(
    conn: &mut PgConnection,
    task_id: Uuid,
    user_id: Uuid,
) -> Result<Option<Task>> {
    let task_row_opt = query_as_wrapper::<Task>(
        "SELECT *
        FROM app.tasks
        WHERE id = $1 AND created_by = $2 AND deleted_at IS NULL",
    )
    .bind(task_id)
    .bind(user_id)
    .fetch_optional(conn.as_mut())
    .await?;

    match task_row_opt {
        None => Ok(None),
        Some(mut task_row) => {
            let tags = query_as_wrapper::<Tag>(
                "SELECT tg.*
                FROM app.task_tags tt
                JOIN app.tags tg ON tt.tag_id = tg.id
                WHERE tt.task_id = $1",
            )
            .bind(task_id)
            .fetch_all(conn.as_mut())
            .await?;

            task_row.tags = tags;

            Ok(Some(task_row))
        }
    }
}

/// Inserts task into database
///
/// # Arguments
///
/// * `pool`: Database connection pool
/// * `user_id`: User ID of task owner
/// * `insert_task`: Task being inserted
///
/// # Returns
///
/// Created task
pub async fn insert_task(pool: PgPool, user_id: Uuid, insert_task: TaskCreate) -> Result<Task> {
    let mut tx = pool.begin().await?;
    let task = insert_task_inner(&mut tx, user_id, insert_task).await?;
    tx.commit().await?;

    Ok(task)
}

/// Internal function for `insert_task`
///
/// Only used internally
async fn insert_task_inner(
    conn: &mut PgConnection,
    user_id: Uuid,
    insert_task: TaskCreate,
) -> Result<Task> {
    let mut task_row = query_as_wrapper::<Task>(
        "INSERT INTO app.tasks (title, notes, start_dt, has_time, deadline, created_by)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING *",
    )
    .bind(insert_task.title)
    .bind(insert_task.notes)
    .bind(insert_task.start.as_ref().and_then(|s| match s.as_at() {
        Some(dt) => Some(dt),
        None => match s.as_on() {
            Some(d) => Some(d.and_hms_opt(0, 0, 0).unwrap().and_utc()),
            None => unreachable!(),
        },
    }))
    .bind(
        insert_task
            .start
            .as_ref()
            .is_some_and(|s| s.as_at().is_some()),
    )
    .bind(insert_task.deadline)
    .bind(user_id)
    .fetch_one(conn.as_mut())
    .await?;

    if !insert_task.tags.is_empty() {
        task_row.tags =
            update_task_tags_inner(conn, task_row.id, user_id, insert_task.tags).await?;
    }

    Ok(task_row)
}

/// Update task in database
///
/// This function is not yet idempotent, but will be in the future:
/// multiple calls will only update the first time
///
/// # Arguments
///
/// * `pool`: Database connection pool
/// * `task_id`: ID of task being updated
/// * `user_id`: User ID of task owner
/// * `update_task`: Updated task
///
/// # Returns
///
/// Updated task
pub async fn update_task(
    pool: PgPool,
    task_id: Uuid,
    user_id: Uuid,
    update_task: TaskCreate,
) -> Result<Task> {
    // TODO: rename TaskCreate?
    let mut tx = pool.begin().await?;
    let task = update_task_inner(&mut tx, task_id, user_id, update_task).await?;
    tx.commit().await?;

    Ok(task)
}

/// Internal function for `update_task`
///
/// Only used internally
async fn update_task_inner(
    conn: &mut PgConnection,
    task_id: Uuid,
    user_id: Uuid,
    update_task: TaskCreate,
) -> Result<Task> {
    // TOOD: make this truly idempotent, don't update if no actual change is made
    let task_opt = query_as_wrapper::<Task>(
        "UPDATE app.tasks SET
        (updated_at, title, notes, start_dt, has_time, deadline) =
        (CURRENT_TIMESTAMP, $3, $4, $5, $6, $7)
        WHERE id = $1 AND created_by = $2 AND deleted_at IS NULL
        RETURNING *",
    )
    .bind(task_id)
    .bind(user_id)
    .bind(update_task.title)
    .bind(update_task.notes)
    .bind(update_task.start.as_ref().and_then(|s| match s.as_at() {
        Some(dt) => Some(dt),
        None => match s.as_on() {
            Some(d) => Some(d.and_hms_opt(0, 0, 0).unwrap().and_utc()),
            None => unreachable!(),
        },
    }))
    .bind(
        update_task
            .start
            .as_ref()
            .is_some_and(|s| s.as_at().is_some()),
    )
    .bind(update_task.deadline)
    .fetch_optional(conn.as_mut())
    .await?;

    if task_opt.is_none() {
        return Err(Error::Application(ApplicationError::TaskNotFound));
    }

    if let Err(err) = update_task_tags_inner(conn, task_id, user_id, update_task.tags).await {
        assert!(!matches!(
            err,
            Error::Application(ApplicationError::TaskNotFound)
        ));

        return Err(err);
    }

    Ok(select_task_inner(conn, task_id, user_id)
        .await?
        .expect("task was just updated"))
}

/// Delete task from database
///
/// This function is idempotent:
/// multiple calls will only delete the first time
///
/// If the task doesn't exist, return as if success.
///
/// Idea: 'make sure client can not see task'
///
/// # Arguments
///
/// * `pool`: Database connection pool
/// * `task_id`: ID of task being deleted
/// * `user_id`: User ID of task owner
///
/// # Returns
///
/// Unit `()`
pub async fn delete_task(pool: PgPool, task_id: Uuid, user_id: Uuid) -> Result<()> {
    let mut tx = pool.begin().await?;
    delete_task_inner(&mut tx, task_id, user_id).await?;
    tx.commit().await?;

    Ok(())
}

/// Internal function for `delete_task`
///
/// Only used internally
async fn delete_task_inner(conn: &mut PgConnection, task_id: Uuid, user_id: Uuid) -> Result<()> {
    sqlx::query(
        "UPDATE app.tasks SET
        (updated_at, deleted_at) =
        (CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
        WHERE id = $1 AND created_by = $2 AND deleted_at IS NULL",
    )
    .bind(task_id)
    .bind(user_id)
    .execute(conn.as_mut())
    .await?;

    Ok(())
}

/// Restore deleted task in database
///
/// This function is idempotent:
/// multiple calls will only restore the first time
///
/// As opposed to `delete`, this function will error if task does not exist.
///
/// Idea: 'make sure client can see task',
/// but if task does not exist, they never can see it
///
/// # Arguments
///
/// * `pool`: Database connection pool
/// * `task_id`: ID of task being restored
/// * `user_id`: User ID of task owner
///
/// # Returns
///
/// Unit `()`
pub async fn restore_task(pool: PgPool, task_id: Uuid, user_id: Uuid) -> Result<()> {
    let mut tx = pool.begin().await?;
    restore_task_inner(&mut tx, task_id, user_id).await?;
    tx.commit().await?;

    Ok(())
}

/// Internal function for `restore_task`
///
/// Only used internally
async fn restore_task_inner(conn: &mut PgConnection, task_id: Uuid, user_id: Uuid) -> Result<()> {
    // task existence (including deleted)
    if sqlx::query("SELECT 1 FROM app.tasks WHERE id = $1 AND created_by = $2")
        .bind(task_id)
        .bind(user_id)
        .fetch_one(conn.as_mut())
        .await
        .is_err()
    {
        return Err(Error::Application(ApplicationError::TaskNotFound));
    }

    sqlx::query(
        "UPDATE app.tasks SET
        (updated_at, deleted_at) =
        (CURRENT_TIMESTAMP, NULL)
        WHERE id = $1 AND created_by = $2 AND deleted_at IS NOT NULL",
    )
    .bind(task_id)
    .bind(user_id)
    .execute(conn.as_mut())
    .await?;

    Ok(())
}

/// Mark/unmark task as completed in database
///
/// This function is idempotent:
/// multiple calls will only complete/uncomplete the first time
///
/// # Arguments
///
/// * `pool`: Database connection pool
/// * `task_id`: ID of task being marked/unmarked completed
/// * `user_id`: User ID of task owner
///
/// # Returns
///
/// `true`, if task was marked as complete
///
/// `false` if task was marked as not complete
pub async fn complete_task(
    pool: PgPool,
    task_id: Uuid,
    user_id: Uuid,
    completed: bool,
) -> Result<bool> {
    let mut tx = pool.begin().await?;
    complete_task_inner(&mut tx, task_id, user_id, completed).await?;
    tx.commit().await?;

    Ok(completed)
}

/// Internal function for `complete_task`
///
/// Only used internally
async fn complete_task_inner(
    conn: &mut PgConnection,
    task_id: Uuid,
    user_id: Uuid,
    completed: bool,
) -> Result<()> {
    if select_task_inner(conn, task_id, user_id).await?.is_none() {
        return Err(Error::Application(ApplicationError::TaskNotFound));
    }

    let query = if completed {
        "UPDATE app.tasks SET
        (updated_at, completed_at) =
        (CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
        WHERE id = $1
        AND created_by = $2
        AND deleted_at IS NULL
        AND completed_at IS NULL"
    } else {
        "UPDATE app.tasks SET
        (updated_at, completed_at) =
        (CURRENT_TIMESTAMP, NULL)
        WHERE id = $1
        AND created_by = $2
        AND deleted_at IS NULL
        AND completed_at IS NOT NULL"
    };

    sqlx::query(query)
        .bind(task_id)
        .bind(user_id)
        .execute(conn.as_mut())
        .await?;

    Ok(())
}

#[cfg(test)]
mod test_helpers {
    use std::env;

    use chrono::{DateTime, SubsecRound, Utc};
    use sqlx::{Connection, PgConnection, PgPool, postgres::PgPoolOptions};
    use tokio::sync::OnceCell;
    use uuid::Uuid;

    use crate::{
        db::{query_as_wrapper, task::update_task_tags_inner},
        models::{Tag, Task},
        routes::models::{Start, tag::Model as TagCreate, task::Model as TaskCreate},
        run_migration,
    };

    pub const PG_SUBSEC_PREC: u16 = 6;

    static POOL: OnceCell<PgPool> = OnceCell::const_new();
    pub async fn get_pool() -> &'static PgPool {
        POOL.get_or_init(|| async {
            dotenvy::dotenv().ok();

            let url = env::var("MIGRATION_URL").unwrap();
            let mut conn = PgConnection::connect(&url).await.unwrap();

            // migration
            run_migration(&mut conn).await.unwrap();

            // add dummy user
            sqlx::query(
                "INSERT INTO auth.user (\"id\", \"name\", \"email\", \"emailVerified\")
                VALUES ($1, $2, $3, $4)
                ON CONFLICT (\"id\") DO NOTHING;",
            )
            .bind(Uuid::nil())
            .bind("testuser")
            .bind("testuser@email.com")
            .bind(false)
            .execute(&mut conn)
            .await
            .unwrap();

            // testing user
            let url = env::var("DATABASE_URL").unwrap();
            let pool_opts = PgPoolOptions::new().max_connections(1);

            pool_opts.connect(&url).await.unwrap()
        })
        .await
    }

    pub async fn create_test_task(
        conn: &mut PgConnection,
        task: TaskCreate,
        completed_at: Option<DateTime<Utc>>,
        deleted_at: Option<DateTime<Utc>>,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Task {
        let task_id = sqlx::query_scalar(
            "INSERT INTO app.tasks (title, notes, start_dt, has_time, deadline, created_by, completed_at, deleted_at, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10) RETURNING id",
        )
        .bind(task.title.clone())
        .bind(task.notes.clone())
        .bind(task.start.as_ref().and_then(|s| match s.as_at() {
            Some(dt) => Some(dt),
            None => match s.as_on() {
                Some(d) => Some(d.and_hms_opt(0, 0, 0).unwrap().and_utc()),
                None => unreachable!(),
            }
        }))
        .bind(task.start.as_ref().is_some_and(|s| s.as_at().is_some()))
        .bind(task.deadline)
        .bind(Uuid::nil())
        .bind(completed_at)
        .bind(deleted_at)
        .bind(created_at)
        .bind(updated_at)
        .fetch_one(conn.as_mut())
        .await.unwrap();

        if !task.tags.is_empty() {
            update_task_tags_inner(conn, task_id, Uuid::nil(), task.tags.clone())
                .await
                .unwrap();
        }

        let ret_task = get_task(conn, task_id).await;
        assert_eq!(ret_task.title, task.title, "title does not match input");
        assert_eq!(ret_task.notes, task.notes, "notes does not match input");
        if task.start.is_some() {
            assert!(ret_task.start_dt.is_some());
            if let Some(dt) = task.start {
                match dt {
                    Start::On(date) => assert_eq!(ret_task.start_dt.unwrap().date_naive(), date),
                    Start::At(date_time) => assert_eq!(ret_task.start_dt.unwrap(), date_time),
                }
            }
        } else {
            assert!(ret_task.start_dt.is_none())
        }
        assert_eq!(
            ret_task.deadline, task.deadline,
            "deadline does not match input"
        );
        assert_eq!(
            ret_task.tags.len(),
            task.tags.len(),
            "number of tags does not match input"
        );
        assert_eq!(
            ret_task
                .tags
                .iter()
                .map(|tag| tag.id)
                .collect::<Vec<Uuid>>(),
            task.tags,
            "tags don't match input"
        );
        assert_eq!(
            ret_task.created_by,
            Uuid::nil(),
            "created_by does not match input"
        );
        assert_eq!(
            ret_task.completed_at,
            completed_at.map(|date| date.trunc_subsecs(PG_SUBSEC_PREC)),
            "completed_at does not match input"
        );
        assert_eq!(
            ret_task.deleted_at,
            deleted_at.map(|date| date.trunc_subsecs(PG_SUBSEC_PREC)),
            "deleted_at does not match input"
        );
        assert_eq!(
            ret_task.created_at,
            created_at.trunc_subsecs(PG_SUBSEC_PREC),
            "created_at does not match input"
        );
        assert_eq!(
            ret_task.updated_at,
            updated_at.trunc_subsecs(PG_SUBSEC_PREC),
            "updated_at does not match input"
        );

        ret_task
    }

    pub async fn get_task(conn: &mut PgConnection, task_id: Uuid) -> Task {
        let mut task = query_as_wrapper::<Task>(
            "SELECT *
                FROM app.tasks
                WHERE id = $1 AND created_by = $2",
        )
        .bind(task_id)
        .bind(Uuid::nil())
        .fetch_one(conn.as_mut())
        .await
        .unwrap();

        task.tags = query_as_wrapper::<Tag>(
            "SELECT tg.*
                FROM app.task_tags tt
                JOIN app.tags tg ON tt.tag_id = tg.id
                WHERE tt.task_id = $1",
        )
        .bind(task_id)
        .fetch_all(conn.as_mut())
        .await
        .unwrap();

        task
    }

    pub async fn create_test_tag(
        conn: &mut PgConnection,
        tag: TagCreate,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Tag {
        let tag_id = sqlx::query_scalar(
            "INSERT INTO app.tags (label, category, created_by, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5) RETURNING id",
        )
        .bind(tag.label.clone())
        .bind(tag.category.clone())
        .bind(Uuid::nil())
        .bind(created_at)
        .bind(updated_at)
        .fetch_one(conn.as_mut())
        .await
        .unwrap();

        let ret_tag = get_tag(conn, tag_id).await;
        assert_eq!(ret_tag.label, tag.label, "label does not match input");
        assert_eq!(
            ret_tag.category, tag.category,
            "category does not match input"
        );
        assert_eq!(
            ret_tag.created_by,
            Uuid::nil(),
            "created_by does not match input"
        );
        assert_eq!(
            ret_tag.created_at,
            created_at.trunc_subsecs(PG_SUBSEC_PREC),
            "created_at does not match input"
        );
        assert_eq!(
            ret_tag.updated_at,
            updated_at.trunc_subsecs(PG_SUBSEC_PREC),
            "updated_at does not match input"
        );

        ret_tag
    }

    pub async fn get_tag(conn: &mut PgConnection, tag_id: Uuid) -> Tag {
        query_as_wrapper::<Tag>(
            "SELECT *
        FROM app.tags
        WHERE id = $1 AND created_by = $2",
        )
        .bind(tag_id)
        .bind(Uuid::nil())
        .fetch_one(conn.as_mut())
        .await
        .unwrap()
    }
}

#[cfg(test)]
mod query_tests {
    use std::{collections::HashSet, time::Duration};

    use chrono::{NaiveDate, NaiveDateTime, NaiveTime, Utc};
    use sqlx::PgConnection;
    use tokio::test;
    use uuid::Uuid;

    use super::{
        TaskQueryOptions, query_tasks_inner,
        test_helpers::{create_test_tag, create_test_task, get_pool},
    };
    use crate::{
        com::constants::MAX_LIMIT,
        db::{
            filters::{DateBound, DateFilter, TaskSort},
            query_as_wrapper,
        },
        models::{Tag, Task},
        routes::models::{SortOrder, Start, tag::Model as TagCreate, task::Model as TaskCreate},
    };

    const DATE_YEAR: i32 = 2027;
    const START_MONTH: u32 = 1;
    const DEADLINE_MONTH: u32 = 2;
    const DELETED_MONTH: u32 = 3;

    fn create_default_opts() -> TaskQueryOptions {
        TaskQueryOptions {
            limit: None,
            offset: None,
            sort_order: TaskSort::default(),
            completed: false,
            deleted: false,
            start_filter: None,
            deadline_filter: None,
        }
    }

    async fn create_test_data(tx: &mut PgConnection) {
        let mut num_tasks = 0;
        let mut num_tags = 0;

        // Create empty tasks
        let base_time = Utc::now();
        for i in 1..=10 {
            let title = format!("Test Task {}", i);

            let task = TaskCreate {
                title: title.clone(),
                notes: Some(format!("Notes for '{}'", title)),
                ..Default::default()
            };

            create_test_task(
                tx,
                task,
                None,
                None,
                base_time + Duration::from_secs(i),
                base_time + Duration::from_hours(1) + Duration::from_secs(60 - i),
            )
            .await;
            num_tasks += 1;
        }

        assert_eq!(num_tasks, 10);
        assert_eq!(num_tags, 0);

        // Create start date tasks
        let base_time = Utc::now() + Duration::from_hours(1);
        for i in 1..=10 {
            let title = format!("Test Task SD{}", i);
            let date = NaiveDate::from_ymd_opt(DATE_YEAR, START_MONTH, i).unwrap();

            let task = TaskCreate {
                title: title.clone(),
                notes: Some(format!("Notes '{}'", title)),
                start: Some(Start::On(date)),
                ..Default::default()
            };

            create_test_task(
                tx,
                task,
                None,
                None,
                base_time + Duration::from_secs(i as u64),
                base_time + Duration::from_hours(1) + Duration::from_secs(60 - i as u64),
            )
            .await;
            num_tasks += 1;
        }

        assert_eq!(num_tasks, 20);
        assert_eq!(num_tags, 0);

        // Create start datetime tasks
        let base_time = Utc::now() + Duration::from_hours(2);
        for i in 1..=10 {
            let title = format!("Test Task SDt{}", i);
            let date = NaiveDateTime::new(
                NaiveDate::from_ymd_opt(DATE_YEAR, START_MONTH, i).unwrap(),
                NaiveTime::from_hms_opt(12, 00, 00).unwrap(),
            );

            let task = TaskCreate {
                title: title.clone(),
                notes: Some(format!("Notes '{}'", title)),
                start: Some(Start::At(date.and_utc())),
                ..Default::default()
            };

            create_test_task(
                tx,
                task,
                None,
                None,
                base_time + Duration::from_secs(i as u64),
                base_time + Duration::from_hours(1) + Duration::from_secs(60 - i as u64),
            )
            .await;
            num_tasks += 1;
        }

        assert_eq!(num_tasks, 30);
        assert_eq!(num_tags, 0);

        // Create deadline tasks
        let base_time = Utc::now() + Duration::from_hours(3);
        for i in 1..=10 {
            let title = format!("Test Task Dl{}", i);
            let date = NaiveDate::from_ymd_opt(DATE_YEAR, DEADLINE_MONTH, i).unwrap();

            let task = TaskCreate {
                title: title.clone(),
                notes: Some(format!("Notes '{}'", title)),
                deadline: Some(date),
                ..Default::default()
            };

            create_test_task(
                tx,
                task,
                None,
                None,
                base_time + Duration::from_secs(i as u64),
                base_time + Duration::from_hours(1) + Duration::from_secs(60 - i as u64),
            )
            .await;
            num_tasks += 1;
        }

        assert_eq!(num_tasks, 40);
        assert_eq!(num_tags, 0);

        // Create completed tasks
        let base_time = Utc::now() + Duration::from_hours(4);
        for i in 1..=10 {
            let title = format!("Test Task Comp{}", i);
            let date = NaiveDate::from_ymd_opt(DATE_YEAR, DELETED_MONTH, i).unwrap();

            let task = TaskCreate {
                title: title.clone(),
                notes: Some(format!("Notes '{}'", title)),
                deadline: Some(date),
                ..Default::default()
            };

            create_test_task(
                tx,
                task,
                Some(base_time + Duration::from_hours(2) + Duration::from_secs(i as u64)),
                None,
                base_time + Duration::from_secs(i as u64),
                base_time + Duration::from_hours(1) + Duration::from_secs(60 - i as u64),
            )
            .await;
            num_tasks += 1;
        }

        assert_eq!(num_tasks, 50);
        assert_eq!(num_tags, 0);

        // Create deleted tasks
        let base_time = Utc::now() + Duration::from_hours(5);
        for i in 1..=10 {
            let title = format!("Test Task Del{}", i);
            let date = NaiveDate::from_ymd_opt(DATE_YEAR, DELETED_MONTH, i).unwrap();

            let task = TaskCreate {
                title: title.clone(),
                notes: Some(format!("Notes '{}'", title)),
                deadline: Some(date),
                ..Default::default()
            };

            create_test_task(
                tx,
                task,
                None,
                Some(base_time + Duration::from_hours(2) + Duration::from_secs(i as u64)),
                base_time + Duration::from_secs(i as u64),
                base_time + Duration::from_hours(1) + Duration::from_secs(60 - i as u64),
            )
            .await;
            num_tasks += 1;
        }

        assert_eq!(num_tasks, 60);
        assert_eq!(num_tags, 0);

        // Create priority tags
        let base_time = Utc::now() + Duration::from_hours(6);
        let low_tag = create_test_tag(
            tx,
            TagCreate {
                label: "Low".to_string(),
                category: Some("Priority".to_string()),
            },
            base_time,
            base_time,
        )
        .await;
        let mid_tag = create_test_tag(
            tx,
            TagCreate {
                label: "Mid".to_string(),
                category: Some("Priority".to_string()),
            },
            base_time,
            base_time,
        )
        .await;
        let high_tag = create_test_tag(
            tx,
            TagCreate {
                label: "High".to_string(),
                category: Some("Priority".to_string()),
            },
            base_time,
            base_time,
        )
        .await;
        num_tags += 3;

        assert_eq!(num_tasks, 60);
        assert_eq!(num_tags, 3);

        // Create priority tasks
        let base_time = Utc::now() + Duration::from_hours(6);
        for i in 1..=12 {
            let tag = match i {
                1..=4 => low_tag.clone(),
                5..=8 => mid_tag.clone(),
                9..=12 => high_tag.clone(),
                _ => unreachable!(),
            };

            create_test_task(
                tx,
                TaskCreate {
                    title: format!("Test Tag Prio{}", i),
                    tags: vec![tag.id],
                    ..Default::default()
                },
                None,
                None,
                base_time + Duration::from_secs(i as u64),
                base_time + Duration::from_hours(1) + Duration::from_secs(60 - i as u64),
            )
            .await;
            num_tasks += 1;
        }

        assert_eq!(num_tasks, 72);
        assert_eq!(num_tags, 3);

        // Create workflow tags
        let base_time = Utc::now() + Duration::from_hours(6);
        let backlog_tag = create_test_tag(
            tx,
            TagCreate {
                label: "Backlog".to_string(),
                category: Some("Workflow".to_string()),
            },
            base_time,
            base_time,
        )
        .await;
        let todo_tag = create_test_tag(
            tx,
            TagCreate {
                label: "Todo".to_string(),
                category: Some("Workflow".to_string()),
            },
            base_time,
            base_time,
        )
        .await;
        let in_progress_tag = create_test_tag(
            tx,
            TagCreate {
                label: "In-progress".to_string(),
                category: Some("Workflow".to_string()),
            },
            base_time,
            base_time,
        )
        .await;
        let completed_tag = create_test_tag(
            tx,
            TagCreate {
                label: "Completed".to_string(),
                category: Some("Workflow".to_string()),
            },
            base_time,
            base_time,
        )
        .await;
        num_tags += 4;

        assert_eq!(num_tasks, 72);
        assert_eq!(num_tags, 7);

        // Create workflow tasks
        for i in 1..=16 {
            let tag = match i {
                1..=4 => backlog_tag.clone(),
                5..=8 => todo_tag.clone(),
                9..=12 => in_progress_tag.clone(),
                13..=16 => completed_tag.clone(),
                _ => unreachable!(),
            };

            create_test_task(
                tx,
                TaskCreate {
                    title: format!("Test Tag Work{}", i),
                    tags: vec![tag.id],
                    ..Default::default()
                },
                None,
                None,
                base_time + Duration::from_secs(i as u64),
                base_time + Duration::from_hours(1) + Duration::from_secs(60 - i as u64),
            )
            .await;
            num_tasks += 1;
        }

        assert_eq!(num_tasks, 88);
        assert_eq!(num_tags, 7);

        let tasks = query_as_wrapper::<Task>("SELECT * FROM app.tasks WHERE created_by = $1")
            .bind(Uuid::nil())
            .fetch_all(tx.as_mut())
            .await
            .unwrap();
        assert_eq!(tasks.len(), num_tasks);
        let tags = query_as_wrapper::<Tag>("SELECT * FROM app.tags WHERE created_by = $1")
            .bind(Uuid::nil())
            .fetch_all(tx.as_mut())
            .await
            .unwrap();
        assert_eq!(tags.len(), num_tags);
    }

    #[test]
    async fn base_query() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        // create data
        create_test_data(&mut tx).await;

        let opts = create_default_opts();

        let res = query_tasks_inner(&mut tx, Uuid::nil(), opts).await;
        assert!(res.is_ok());
        let tasks = res.unwrap();

        // default sort is updated_at descending
        assert!(tasks.is_sorted_by(|a, b| a.updated_at >= b.updated_at));

        for task in tasks {
            // no deleted tasks
            assert!(task.deleted_at.is_none());
        }
    }

    #[test]
    async fn updated_ascending() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        // create data
        create_test_data(&mut tx).await;

        let mut opts = create_default_opts();
        opts.sort_order = TaskSort::Updated(SortOrder::Ascending);

        let res = query_tasks_inner(&mut tx, Uuid::nil(), opts).await;
        assert!(res.is_ok());
        let tasks = res.unwrap();

        // ensure tasks is sorted by updated_at ascending
        assert!(tasks.is_sorted_by(|a, b| a.updated_at <= b.updated_at));
    }

    #[test]
    async fn created_descending() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        // create data
        create_test_data(&mut tx).await;

        let mut opts = create_default_opts();
        opts.sort_order = TaskSort::Created(SortOrder::Descending);

        let res = query_tasks_inner(&mut tx, Uuid::nil(), opts).await;
        assert!(res.is_ok());
        let tasks = res.unwrap();

        // ensure tasks is sorted by created_at descending
        assert!(tasks.is_sorted_by(|a, b| a.created_at >= b.created_at));
    }

    #[test]
    async fn created_ascending() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        // create data
        create_test_data(&mut tx).await;

        let mut opts = create_default_opts();
        opts.sort_order = TaskSort::Created(SortOrder::Ascending);

        let res = query_tasks_inner(&mut tx, Uuid::nil(), opts).await;
        assert!(res.is_ok());
        let tasks = res.unwrap();

        // ensure tasks is sorted by created_at ascending
        assert!(tasks.is_sorted_by(|a, b| a.created_at <= b.created_at));
    }

    #[test]
    async fn title_ascending() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        // create data
        create_test_data(&mut tx).await;

        let mut opts = create_default_opts();
        opts.sort_order = TaskSort::Title(SortOrder::Ascending);

        let res = query_tasks_inner(&mut tx, Uuid::nil(), opts).await;
        assert!(res.is_ok());
        let tasks = res.unwrap();

        // ensure tasks is sorted by title ascending
        assert!(tasks.is_sorted_by(|a, b| a.title <= b.title));
    }

    #[test]
    async fn title_descending() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        // create data
        create_test_data(&mut tx).await;

        let mut opts = create_default_opts();
        opts.sort_order = TaskSort::Title(SortOrder::Descending);

        let res = query_tasks_inner(&mut tx, Uuid::nil(), opts).await;
        assert!(res.is_ok());
        let tasks = res.unwrap();

        // ensure tasks is sorted by title descending
        assert!(tasks.is_sorted_by(|a, b| a.title >= b.title));
    }

    #[test]
    async fn start_ascending() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        // create data
        create_test_data(&mut tx).await;

        let mut opts = create_default_opts();
        opts.sort_order = TaskSort::Start(SortOrder::Ascending);

        let res = query_tasks_inner(&mut tx, Uuid::nil(), opts).await;
        assert!(res.is_ok());
        let tasks = res.unwrap();

        // ensure tasks is sorted by start ascending
        let mut seen_nulls = false;
        assert!(tasks.is_sorted_by(|a, b| {
            if a.start_dt.is_some() && b.start_dt.is_none() {
                // if encountering first transition from values to NULL
                seen_nulls = true;

                return true;
            }
            if seen_nulls && (a.start_dt.is_some() || b.start_dt.is_some()) {
                // if we are in the NULL section, we should not see anymore dates that are not NULL
                panic!("found dates within NULL section of sorted tasks")
            }

            a.start_dt <= b.start_dt
        }));
    }

    #[test]
    async fn start_descending() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        // create data
        create_test_data(&mut tx).await;

        let mut opts = create_default_opts();
        opts.sort_order = TaskSort::Title(SortOrder::Descending);

        let res = query_tasks_inner(&mut tx, Uuid::nil(), opts).await;
        assert!(res.is_ok());
        let tasks = res.unwrap();

        // ensure tasks is sorted by start descending
        assert!(tasks.is_sorted_by(|a, b| a.title >= b.title));
    }

    #[test]
    async fn deadline_ascending() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        // create data
        create_test_data(&mut tx).await;

        let mut opts = create_default_opts();
        opts.sort_order = TaskSort::Deadline(SortOrder::Ascending);

        let res = query_tasks_inner(&mut tx, Uuid::nil(), opts).await;
        assert!(res.is_ok());
        let tasks = res.unwrap();

        // ensure tasks is sorted by deadline ascending
        let mut seen_nulls = false;
        assert!(tasks.is_sorted_by(|a, b| {
            if a.deadline.is_some() && b.deadline.is_none() {
                // if encountering first transition from values to NULL
                seen_nulls = true;

                return true;
            }
            if seen_nulls && (a.deadline.is_some() || b.deadline.is_some()) {
                // if we are in the NULL section, we should not see anymore dates that are not NULL
                panic!("found dates within NULL section of sorted tasks")
            }

            a.deadline <= b.deadline
        }));
    }

    #[test]
    async fn deadline_descending() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        // create data
        create_test_data(&mut tx).await;

        let mut opts = create_default_opts();
        opts.sort_order = TaskSort::Deadline(SortOrder::Descending);

        let res = query_tasks_inner(&mut tx, Uuid::nil(), opts).await;
        assert!(res.is_ok());
        let tasks = res.unwrap();

        // ensure tasks is sorted by deadline descending
        assert!(tasks.is_sorted_by(|a, b| a.deadline >= b.deadline));
    }

    #[test]
    async fn limit() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        // create data
        create_test_data(&mut tx).await;

        for i in 1..=20 {
            let mut opts = create_default_opts();
            opts.limit = Some(i);

            let res = query_tasks_inner(&mut tx, Uuid::nil(), opts).await;
            assert!(res.is_ok());
            let tasks = res.unwrap();
            assert_eq!(tasks.len(), i as usize);
        }
    }

    #[test]
    async fn limit_0() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        // create data
        create_test_data(&mut tx).await;

        let mut opts = create_default_opts();
        opts.limit = Some(0);

        assert!(query_tasks_inner(&mut tx, Uuid::nil(), opts).await.is_ok());
    }

    #[test]
    async fn limit_absurdly_large() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        // create data
        create_test_data(&mut tx).await;

        let mut opts = create_default_opts();
        opts.limit = Some(i64::MAX);

        let res = query_tasks_inner(&mut tx, Uuid::nil(), opts).await;
        assert!(res.is_ok());

        let tasks = res.unwrap();
        assert!(tasks.len() <= MAX_LIMIT as usize);
    }

    #[test]
    async fn limit_negative() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        // create data
        create_test_data(&mut tx).await;

        let mut opts = create_default_opts();
        opts.limit = Some(-1);

        assert!(query_tasks_inner(&mut tx, Uuid::nil(), opts).await.is_ok());

        let mut opts = create_default_opts();
        opts.limit = Some(-50);

        assert!(query_tasks_inner(&mut tx, Uuid::nil(), opts).await.is_ok());

        let mut opts = create_default_opts();
        opts.limit = Some(i64::MIN);

        assert!(query_tasks_inner(&mut tx, Uuid::nil(), opts).await.is_ok());
    }

    #[test]
    async fn limit_with_lots_of_data() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = Utc::now();

        // create lots of test data
        for i in 0..400 {
            create_test_task(
                &mut tx,
                TaskCreate {
                    title: format!("Test Task {}", i),
                    ..Default::default()
                },
                None,
                None,
                base_time,
                base_time,
            )
            .await;
        }

        let mut opts = create_default_opts();
        opts.limit = Some(i64::MAX);

        let res = query_tasks_inner(&mut tx, Uuid::nil(), opts).await;
        assert!(res.is_ok());

        let tasks = res.unwrap();
        assert_eq!(tasks.len(), MAX_LIMIT as usize);
    }

    #[test]
    async fn limit_with_paging_offset() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        // create data
        create_test_data(&mut tx).await;

        let limit = 5;

        // keep paging until less than 'limit' tasks are return
        let mut i = 0;
        let mut seen = HashSet::new();
        let mut first = true;
        loop {
            let mut opts = create_default_opts();
            opts.limit = Some(limit);
            opts.offset = Some(i * limit);

            let res = query_tasks_inner(&mut tx, Uuid::nil(), opts).await;
            assert!(res.is_ok());
            let tasks = res.unwrap();
            if first {
                assert!(!tasks.is_empty(), "must have data to test on");
                first = false;
            }

            assert!(tasks.len() <= limit as usize);
            for task in &tasks {
                // no deleted tasks
                assert!(task.deleted_at.is_none());

                assert!(seen.insert(task.id), "duplicate task encountered");
            }
            seen.extend(tasks.iter().map(|t| t.id));

            i += 1;

            if tasks.len() < limit as usize {
                break;
            }
        }

        // perform one more query to ensure that the end has been reached
        let mut opts = create_default_opts();
        opts.limit = Some(limit);
        opts.offset = Some(i * limit);

        let res = query_tasks_inner(&mut tx, Uuid::nil(), opts).await;
        assert!(res.is_ok());
        let tasks = res.unwrap();
        assert!(tasks.is_empty());
    }

    #[test]
    async fn offset_absurdly_large() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        // create data
        create_test_data(&mut tx).await;

        let mut opts = create_default_opts();
        opts.offset = Some(i64::MAX);

        assert!(query_tasks_inner(&mut tx, Uuid::nil(), opts).await.is_ok());
    }

    #[test]
    async fn offset_negative() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        // create data
        create_test_data(&mut tx).await;

        let mut opts = create_default_opts();
        opts.offset = Some(-1);

        assert!(query_tasks_inner(&mut tx, Uuid::nil(), opts).await.is_ok());

        let mut opts = create_default_opts();
        opts.offset = Some(-50);

        assert!(query_tasks_inner(&mut tx, Uuid::nil(), opts).await.is_ok());

        let mut opts = create_default_opts();
        opts.offset = Some(i64::MIN);

        assert!(query_tasks_inner(&mut tx, Uuid::nil(), opts).await.is_ok());
    }

    #[test]
    async fn offset_without_limits() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        // create data
        create_test_data(&mut tx).await;

        let mut opts = create_default_opts();
        opts.offset = Some(20);

        assert!(query_tasks_inner(&mut tx, Uuid::nil(), opts).await.is_ok());
    }

    // TODO: add sort order to completed
    #[test]
    async fn filter_completed() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        // create data
        create_test_data(&mut tx).await;

        let mut opts = create_default_opts();
        opts.completed = true;

        let res = query_tasks_inner(&mut tx, Uuid::nil(), opts).await;
        assert!(res.is_ok());
        let tasks = res.unwrap();
        assert!(!tasks.is_empty(), "must have data to test on");

        for task in tasks {
            assert!(task.completed_at.is_some());
        }
    }

    // TODO: add sort order to deleted
    #[test]
    async fn filter_deleted() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        // create data
        create_test_data(&mut tx).await;

        let mut opts = create_default_opts();
        opts.deleted = true;

        let res = query_tasks_inner(&mut tx, Uuid::nil(), opts).await;
        assert!(res.is_ok());
        let tasks = res.unwrap();
        assert!(!tasks.is_empty(), "must have data to test on");

        for task in tasks {
            assert!(task.deleted_at.is_some());
        }
    }

    #[test]
    async fn filter_has_start() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        // create data
        create_test_data(&mut tx).await;

        let mut opts = create_default_opts();
        opts.start_filter = Some(DateFilter::Exists(true));

        let res = query_tasks_inner(&mut tx, Uuid::nil(), opts).await;
        assert!(res.is_ok());
        let tasks = res.unwrap();
        assert!(!tasks.is_empty(), "must have data to test on");

        for task in tasks {
            // no deleted tasks
            assert!(task.deleted_at.is_none());

            assert!(task.start_dt.is_some());
        }

        let mut opts = create_default_opts();
        opts.start_filter = Some(DateFilter::Exists(false));

        let res = query_tasks_inner(&mut tx, Uuid::nil(), opts).await;
        assert!(res.is_ok());
        let tasks = res.unwrap();
        assert!(!tasks.is_empty(), "must have data to test on");

        for task in tasks {
            // no deleted tasks
            assert!(task.deleted_at.is_none());

            assert!(task.start_dt.is_none());
        }
    }

    #[test]
    async fn filter_start_on() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        // create data
        create_test_data(&mut tx).await;

        let test_date = NaiveDate::from_ymd_opt(DATE_YEAR, START_MONTH, 5).unwrap();

        let mut opts = create_default_opts();
        opts.start_filter = Some(DateFilter::On(test_date));

        let res = query_tasks_inner(&mut tx, Uuid::nil(), opts).await;
        assert!(res.is_ok());
        let tasks = res.unwrap();
        assert!(!tasks.is_empty(), "must have data to test on");

        for task in tasks {
            // no deleted tasks
            assert!(task.deleted_at.is_none());

            assert!(task.start_dt.is_some());
            assert_eq!(task.start_dt.unwrap().date_naive(), test_date);
        }
    }

    #[test]
    async fn filter_start_not_on() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        // create data
        create_test_data(&mut tx).await;

        let test_date = NaiveDate::from_ymd_opt(DATE_YEAR, START_MONTH, 5).unwrap();

        let mut opts = create_default_opts();
        opts.start_filter = Some(DateFilter::NotOn(test_date));

        let res = query_tasks_inner(&mut tx, Uuid::nil(), opts).await;
        assert!(res.is_ok());
        let tasks = res.unwrap();
        assert!(!tasks.is_empty(), "must have data to test on");

        for task in tasks {
            // no deleted tasks
            assert!(task.deleted_at.is_none());

            if let Some(dt) = task.start_dt {
                assert_ne!(dt.date_naive(), test_date);
            }
        }
    }

    #[test]
    async fn filter_start_after_excl() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        // create data
        create_test_data(&mut tx).await;

        let test_date = NaiveDate::from_ymd_opt(DATE_YEAR, START_MONTH, 5).unwrap();

        let mut opts = create_default_opts();
        opts.start_filter = Some(DateFilter::StartRange(DateBound::Exclusive(test_date)));

        let res = query_tasks_inner(&mut tx, Uuid::nil(), opts).await;
        assert!(res.is_ok());
        let tasks = res.unwrap();
        assert!(!tasks.is_empty(), "must have data to test on");

        for task in tasks {
            // no deleted tasks
            assert!(task.deleted_at.is_none());

            assert!(task.start_dt.is_some());

            if let Some(dt) = task.start_dt {
                assert!(dt.date_naive() > test_date);
            }
        }
    }

    #[test]
    async fn filter_start_after_incl() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        // create data
        create_test_data(&mut tx).await;

        let test_date = NaiveDate::from_ymd_opt(DATE_YEAR, START_MONTH, 5).unwrap();

        let mut opts = create_default_opts();
        opts.start_filter = Some(DateFilter::StartRange(DateBound::Inclusive(test_date)));

        let res = query_tasks_inner(&mut tx, Uuid::nil(), opts).await;
        assert!(res.is_ok());
        let tasks = res.unwrap();
        assert!(!tasks.is_empty(), "must have data to test on");

        for task in tasks {
            // no deleted tasks
            assert!(task.deleted_at.is_none());

            assert!(task.start_dt.is_some());

            if let Some(dt) = task.start_dt {
                assert!(dt.date_naive() >= test_date);
            }
        }
    }

    #[test]
    async fn filter_start_before_excl() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        // create data
        create_test_data(&mut tx).await;

        let test_date = NaiveDate::from_ymd_opt(DATE_YEAR, START_MONTH, 5).unwrap();

        let mut opts = create_default_opts();
        opts.start_filter = Some(DateFilter::EndRange(DateBound::Exclusive(test_date)));

        let res = query_tasks_inner(&mut tx, Uuid::nil(), opts).await;
        assert!(res.is_ok());
        let tasks = res.unwrap();
        assert!(!tasks.is_empty(), "must have data to test on");

        for task in tasks {
            // no deleted tasks
            assert!(task.deleted_at.is_none());

            assert!(task.start_dt.is_some());

            if let Some(dt) = task.start_dt {
                assert!(dt.date_naive() < test_date);
            }
        }
    }

    #[test]
    async fn filter_start_before_incl() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        // create data
        create_test_data(&mut tx).await;

        let test_date = NaiveDate::from_ymd_opt(DATE_YEAR, START_MONTH, 5).unwrap();

        let mut opts = create_default_opts();
        opts.start_filter = Some(DateFilter::EndRange(DateBound::Inclusive(test_date)));

        let res = query_tasks_inner(&mut tx, Uuid::nil(), opts).await;
        assert!(res.is_ok());
        let tasks = res.unwrap();
        assert!(!tasks.is_empty(), "must have data to test on");

        for task in tasks {
            // no deleted tasks
            assert!(task.deleted_at.is_none());

            assert!(task.start_dt.is_some());

            if let Some(dt) = task.start_dt {
                assert!(dt.date_naive() <= test_date);
            }
        }
    }

    #[test]
    async fn filter_start_between_excl() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        // create data
        create_test_data(&mut tx).await;

        let test_date_min = NaiveDate::from_ymd_opt(DATE_YEAR, START_MONTH, 3).unwrap();
        let test_date_max = NaiveDate::from_ymd_opt(DATE_YEAR, START_MONTH, 6).unwrap();

        let mut opts = create_default_opts();
        opts.start_filter = Some(DateFilter::Range(
            DateBound::Exclusive(test_date_min),
            DateBound::Exclusive(test_date_max),
        ));

        let res = query_tasks_inner(&mut tx, Uuid::nil(), opts).await;
        assert!(res.is_ok());
        let tasks = res.unwrap();
        assert!(!tasks.is_empty(), "must have data to test on");

        for task in tasks {
            // no deleted tasks
            assert!(task.deleted_at.is_none());

            assert!(task.start_dt.is_some());

            if let Some(dt) = task.start_dt {
                assert!(dt.date_naive() > test_date_min && dt.date_naive() < test_date_max);
            }
        }
    }

    #[test]
    async fn filter_start_between_incl() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        // create data
        create_test_data(&mut tx).await;

        let test_date_min = NaiveDate::from_ymd_opt(DATE_YEAR, START_MONTH, 3).unwrap();
        let test_date_max = NaiveDate::from_ymd_opt(DATE_YEAR, START_MONTH, 6).unwrap();

        let mut opts = create_default_opts();
        opts.start_filter = Some(DateFilter::Range(
            DateBound::Inclusive(test_date_min),
            DateBound::Inclusive(test_date_max),
        ));

        let res = query_tasks_inner(&mut tx, Uuid::nil(), opts).await;
        assert!(res.is_ok());
        let tasks = res.unwrap();
        assert!(!tasks.is_empty(), "must have data to test on");

        for task in tasks {
            // no deleted tasks
            assert!(task.deleted_at.is_none());

            assert!(task.start_dt.is_some());

            if let Some(dt) = task.start_dt {
                assert!(dt.date_naive() >= test_date_min && dt.date_naive() <= test_date_max);
            }
        }
    }

    #[test]
    async fn filter_start_between_combos() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        // create data
        create_test_data(&mut tx).await;

        let test_date_min = NaiveDate::from_ymd_opt(DATE_YEAR, START_MONTH, 3).unwrap();
        let test_date_max = NaiveDate::from_ymd_opt(DATE_YEAR, START_MONTH, 6).unwrap();

        // test exclusive max
        let mut opts = create_default_opts();
        opts.start_filter = Some(DateFilter::Range(
            DateBound::Inclusive(test_date_min),
            DateBound::Exclusive(test_date_max),
        ));

        let res = query_tasks_inner(&mut tx, Uuid::nil(), opts).await;
        assert!(res.is_ok());
        let tasks = res.unwrap();
        assert!(!tasks.is_empty(), "must have data to test on");

        for task in tasks {
            // no deleted tasks
            assert!(task.deleted_at.is_none());

            assert!(task.start_dt.is_some());

            if let Some(dt) = task.start_dt {
                assert!(dt.date_naive() >= test_date_min && dt.date_naive() < test_date_max);
            }
        }

        // text exclusive min
        let mut opts = create_default_opts();
        opts.start_filter = Some(DateFilter::Range(
            DateBound::Exclusive(test_date_min),
            DateBound::Inclusive(test_date_max),
        ));

        let res = query_tasks_inner(&mut tx, Uuid::nil(), opts).await;
        assert!(res.is_ok());
        let tasks = res.unwrap();
        assert!(!tasks.is_empty(), "must have data to test on");

        for task in tasks {
            // no deleted tasks
            assert!(task.deleted_at.is_none());

            assert!(task.start_dt.is_some());

            if let Some(dt) = task.start_dt {
                assert!(dt.date_naive() > test_date_min && dt.date_naive() <= test_date_max);
            }
        }
    }

    #[test]
    async fn filter_has_deadline() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        // create data
        create_test_data(&mut tx).await;

        let mut opts = create_default_opts();
        opts.deadline_filter = Some(DateFilter::Exists(true));

        let res = query_tasks_inner(&mut tx, Uuid::nil(), opts).await;
        assert!(res.is_ok());
        let tasks = res.unwrap();
        assert!(!tasks.is_empty(), "must have data to test on");

        for task in tasks {
            // no deleted tasks
            assert!(task.deleted_at.is_none());

            assert!(task.deadline.is_some());
        }

        let mut opts = create_default_opts();
        opts.deadline_filter = Some(DateFilter::Exists(false));

        let res = query_tasks_inner(&mut tx, Uuid::nil(), opts).await;
        assert!(res.is_ok());
        let tasks = res.unwrap();
        assert!(!tasks.is_empty(), "must have data to test on");

        for task in tasks {
            // no deleted tasks
            assert!(task.deleted_at.is_none());

            assert!(task.deadline.is_none());
        }
    }

    #[test]
    async fn filter_deadline_on() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        // create data
        create_test_data(&mut tx).await;

        let test_date = NaiveDate::from_ymd_opt(DATE_YEAR, DEADLINE_MONTH, 5).unwrap();

        let mut opts = create_default_opts();
        opts.deadline_filter = Some(DateFilter::On(test_date));

        let res = query_tasks_inner(&mut tx, Uuid::nil(), opts).await;
        assert!(res.is_ok());
        let tasks = res.unwrap();
        assert!(!tasks.is_empty(), "must have data to test on");

        for task in tasks {
            // no deleted tasks
            assert!(task.deleted_at.is_none());

            assert_eq!(task.deadline, Some(test_date));
        }
    }

    #[test]
    async fn filter_deadline_not_on() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        // create data
        create_test_data(&mut tx).await;

        let test_date = NaiveDate::from_ymd_opt(DATE_YEAR, DEADLINE_MONTH, 5).unwrap();

        let mut opts = create_default_opts();
        opts.deadline_filter = Some(DateFilter::NotOn(test_date));

        let res = query_tasks_inner(&mut tx, Uuid::nil(), opts).await;
        assert!(res.is_ok());
        let tasks = res.unwrap();
        assert!(!tasks.is_empty(), "must have data to test on");

        for task in tasks {
            // no deleted tasks
            assert!(task.deleted_at.is_none());

            if let Some(date) = task.deadline {
                assert_ne!(date, test_date);
            }
        }
    }

    #[test]
    async fn filter_deadline_after_excl() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        // create data
        create_test_data(&mut tx).await;

        let test_date = NaiveDate::from_ymd_opt(DATE_YEAR, DEADLINE_MONTH, 5).unwrap();

        let mut opts = create_default_opts();
        opts.deadline_filter = Some(DateFilter::StartRange(DateBound::Exclusive(test_date)));

        let res = query_tasks_inner(&mut tx, Uuid::nil(), opts).await;
        assert!(res.is_ok());
        let tasks = res.unwrap();
        assert!(!tasks.is_empty(), "must have data to test on");

        for task in tasks {
            // no deleted tasks
            assert!(task.deleted_at.is_none());

            assert!(task.deadline.is_some());

            assert!(task.deadline.unwrap() > test_date);
        }
    }

    #[test]
    async fn filter_deadline_after_incl() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        // create data
        create_test_data(&mut tx).await;

        let test_date = NaiveDate::from_ymd_opt(DATE_YEAR, DEADLINE_MONTH, 5).unwrap();

        let mut opts = create_default_opts();
        opts.deadline_filter = Some(DateFilter::StartRange(DateBound::Inclusive(test_date)));

        let res = query_tasks_inner(&mut tx, Uuid::nil(), opts).await;
        assert!(res.is_ok());
        let tasks = res.unwrap();
        assert!(!tasks.is_empty(), "must have data to test on");

        for task in tasks {
            // no deleted tasks
            assert!(task.deleted_at.is_none());

            assert!(task.deadline.is_some());

            assert!(task.deadline.unwrap() >= test_date);
        }
    }

    #[test]
    async fn filter_deadline_before_excl() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        // create data
        create_test_data(&mut tx).await;

        let test_date = NaiveDate::from_ymd_opt(DATE_YEAR, DEADLINE_MONTH, 5).unwrap();

        let mut opts = create_default_opts();
        opts.deadline_filter = Some(DateFilter::EndRange(DateBound::Exclusive(test_date)));

        let res = query_tasks_inner(&mut tx, Uuid::nil(), opts).await;
        assert!(res.is_ok());
        let tasks = res.unwrap();
        assert!(!tasks.is_empty(), "must have data to test on");

        for task in tasks {
            // no deleted tasks
            assert!(task.deleted_at.is_none());

            assert!(task.deadline.is_some());

            assert!(task.deadline.unwrap() < test_date);
        }
    }

    #[test]
    async fn filter_deadline_before_incl() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        // create data
        create_test_data(&mut tx).await;

        let test_date = NaiveDate::from_ymd_opt(DATE_YEAR, DEADLINE_MONTH, 5).unwrap();

        let mut opts = create_default_opts();
        opts.deadline_filter = Some(DateFilter::EndRange(DateBound::Inclusive(test_date)));

        let res = query_tasks_inner(&mut tx, Uuid::nil(), opts).await;
        assert!(res.is_ok());
        let tasks = res.unwrap();
        assert!(!tasks.is_empty(), "must have data to test on");

        for task in tasks {
            // no deleted tasks
            assert!(task.deleted_at.is_none());

            assert!(task.deadline.is_some());

            assert!(task.deadline.unwrap() <= test_date);
        }
    }

    #[test]
    async fn filter_deadline_between_excl() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        // create data
        create_test_data(&mut tx).await;

        let test_date_min = NaiveDate::from_ymd_opt(DATE_YEAR, DEADLINE_MONTH, 3).unwrap();
        let test_date_max = NaiveDate::from_ymd_opt(DATE_YEAR, DEADLINE_MONTH, 6).unwrap();

        let mut opts = create_default_opts();
        opts.deadline_filter = Some(DateFilter::Range(
            DateBound::Exclusive(test_date_min),
            DateBound::Exclusive(test_date_max),
        ));

        let res = query_tasks_inner(&mut tx, Uuid::nil(), opts).await;
        assert!(res.is_ok());
        let tasks = res.unwrap();
        assert!(!tasks.is_empty(), "must have data to test on");

        for task in tasks {
            // no deleted tasks
            assert!(task.deleted_at.is_none());

            assert!(task.deadline.is_some());

            assert!(
                task.deadline.unwrap() > test_date_min && task.deadline.unwrap() < test_date_max
            );
        }
    }

    #[test]
    async fn filter_deadline_between_incl() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        // create data
        create_test_data(&mut tx).await;

        let test_date_min = NaiveDate::from_ymd_opt(DATE_YEAR, DEADLINE_MONTH, 3).unwrap();
        let test_date_max = NaiveDate::from_ymd_opt(DATE_YEAR, DEADLINE_MONTH, 6).unwrap();

        let mut opts = create_default_opts();
        opts.deadline_filter = Some(DateFilter::Range(
            DateBound::Inclusive(test_date_min),
            DateBound::Inclusive(test_date_max),
        ));

        let res = query_tasks_inner(&mut tx, Uuid::nil(), opts).await;
        assert!(res.is_ok());
        let tasks = res.unwrap();
        assert!(!tasks.is_empty(), "must have data to test on");

        for task in tasks {
            // no deleted tasks
            assert!(task.deleted_at.is_none());

            assert!(task.deadline.is_some());

            assert!(
                task.deadline.unwrap() >= test_date_min && task.deadline.unwrap() <= test_date_max
            );
        }
    }

    #[test]
    async fn query_returns_tasks_with_tags() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        // create data
        create_test_data(&mut tx).await;

        let mut opts = create_default_opts();
        opts.limit = Some(MAX_LIMIT);

        let res = query_tasks_inner(&mut tx, Uuid::nil(), opts).await;
        assert!(res.is_ok());
        let tasks: Vec<Task> = res
            .unwrap()
            .into_iter()
            .filter(|task| task.title.contains("Prio") || task.title.contains("Work"))
            .collect();
        assert!(!tasks.is_empty(), "must have data to test on");

        for task in tasks {
            assert!(!task.tags.is_empty(), "{:?}", task);
        }
    }

    #[test]
    async fn as_nonexistent_user() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let opts = create_default_opts();

        let res = query_tasks_inner(&mut tx, Uuid::new_v4(), opts).await;
        assert!(res.is_ok());
        let tasks = res.unwrap();
        assert!(tasks.is_empty());
    }

    #[test]
    async fn full_combination() {
        // Use all filters

        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let date_mix = NaiveDate::from_ymd_opt(DATE_YEAR, START_MONTH, 1).unwrap();
        let date_max = NaiveDate::from_ymd_opt(DATE_YEAR, START_MONTH, 7).unwrap();

        let mut opts = create_default_opts();
        opts.limit = Some(5);
        opts.offset = Some(2);
        opts.start_filter = Some(DateFilter::Range(
            DateBound::Inclusive(date_mix),
            DateBound::Inclusive(date_max),
        ));
        opts.deadline_filter = Some(DateFilter::Range(
            DateBound::Inclusive(date_mix),
            DateBound::Inclusive(date_max),
        ));

        assert!(query_tasks_inner(&mut tx, Uuid::nil(), opts).await.is_ok());
    }

    #[test]
    async fn full_combination_nondefault() {
        // Use all filters that are not the default option

        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let date_mix = NaiveDate::from_ymd_opt(DATE_YEAR, START_MONTH, 1).unwrap();
        let date_max = NaiveDate::from_ymd_opt(DATE_YEAR, START_MONTH, 7).unwrap();

        let mut opts = create_default_opts();
        opts.limit = Some(5);
        opts.offset = Some(2);
        opts.completed = true;
        opts.deleted = true;
        opts.start_filter = Some(DateFilter::Range(
            DateBound::Inclusive(date_mix),
            DateBound::Inclusive(date_max),
        ));
        opts.deadline_filter = Some(DateFilter::Range(
            DateBound::Inclusive(date_mix),
            DateBound::Inclusive(date_max),
        ));
        opts.sort_order = TaskSort::Created(SortOrder::Ascending);

        assert!(query_tasks_inner(&mut tx, Uuid::nil(), opts).await.is_ok());
    }
}

#[cfg(test)]
mod select_tests {
    use std::{collections::HashSet, time::Duration};

    use chrono::Utc;
    use tokio::test;
    use uuid::Uuid;

    use super::{
        select_task_inner,
        test_helpers::{create_test_tag, create_test_task, get_pool},
    };
    use crate::routes::models::{tag::Model as TagCreate, task::Model as TaskCreate};

    #[test]
    async fn base_select() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = Utc::now();

        let task = create_test_task(
            &mut tx,
            TaskCreate::default(),
            None,
            None,
            base_time,
            base_time + Duration::from_hours(1),
        )
        .await;

        let res = select_task_inner(&mut tx, task.id, Uuid::nil()).await;
        assert!(res.is_ok());

        let task_opt = res.unwrap();
        assert!(task_opt.is_some());

        let task = task_opt.unwrap();
        assert_eq!(task.id, task.id);
        assert_eq!(task.title, "");
        assert!(task.notes.is_none());
        assert!(task.start_dt.is_none());
        assert!(task.deadline.is_none());
        assert!(task.tags.is_empty());
        assert!(task.completed_at.is_none());
        assert!(task.deleted_at.is_none());
    }

    #[test]
    async fn select_many_different() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = Utc::now();

        let mut seen_ids = HashSet::new();
        for _ in 0..10 {
            let task = create_test_task(
                &mut tx,
                TaskCreate::default(),
                None,
                None,
                base_time,
                base_time + Duration::from_hours(1),
            )
            .await;

            assert!(seen_ids.insert(task.id), "duplicate task encountered");
        }
        assert!(!seen_ids.is_empty());

        for id in seen_ids {
            let res = select_task_inner(&mut tx, id, Uuid::nil()).await;
            assert!(res.is_ok());

            let task_opt = res.unwrap();
            assert!(task_opt.is_some());

            let task = task_opt.unwrap();
            assert_eq!(task.id, id);
            assert_eq!(task.title, "");
            assert!(task.notes.is_none());
            assert!(task.start_dt.is_none());
            assert!(task.deadline.is_none());
            assert!(task.tags.is_empty());
            assert!(task.completed_at.is_none());
            assert!(task.deleted_at.is_none());
        }
    }

    #[test]
    async fn select_with_tag() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = Utc::now();

        let tag = create_test_tag(
            &mut tx,
            TagCreate {
                label: "Test Tag".to_string(),
                category: Some("Testing".to_string()),
            },
            base_time,
            base_time,
        )
        .await;

        let task = create_test_task(
            &mut tx,
            TaskCreate {
                title: "Test Task".to_string(),
                tags: vec![tag.id],
                ..Default::default()
            },
            None,
            None,
            base_time,
            base_time,
        )
        .await;

        let res = select_task_inner(&mut tx, task.id, Uuid::nil()).await;
        assert!(res.is_ok());

        let task_opt = res.unwrap();
        assert!(task_opt.is_some());

        let task = task_opt.unwrap();
        assert_eq!(task.title, "Test Task");

        assert_eq!(task.tags.len(), 1);
        let tag = task.tags.first().unwrap();
        assert_eq!(tag.label, "Test Tag");
        if let Some(ref category) = tag.category {
            assert_eq!(category, "Testing");
        }
    }

    #[test]
    async fn select_with_tags() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = Utc::now();

        let tag_1 = create_test_tag(
            &mut tx,
            TagCreate {
                label: "Test Tag 1".to_string(),
                category: Some("Testing".to_string()),
            },
            base_time,
            base_time,
        )
        .await;
        let tag_2 = create_test_tag(
            &mut tx,
            TagCreate {
                label: "Test Tag 2".to_string(),
                category: Some("Testing".to_string()),
            },
            base_time,
            base_time,
        )
        .await;
        let tag_3 = create_test_tag(
            &mut tx,
            TagCreate {
                label: "Test Tag 3".to_string(),
                category: Some("Testing".to_string()),
            },
            base_time,
            base_time,
        )
        .await;

        let task = create_test_task(
            &mut tx,
            TaskCreate {
                title: "Test Task".to_string(),
                tags: vec![tag_1.id, tag_2.id, tag_3.id],
                ..Default::default()
            },
            None,
            None,
            base_time,
            base_time,
        )
        .await;

        let res = select_task_inner(&mut tx, task.id, Uuid::nil()).await;
        assert!(res.is_ok());

        let task_opt = res.unwrap();
        assert!(task_opt.is_some());

        let task = task_opt.unwrap();
        assert_eq!(task.title, "Test Task");
        assert_eq!(task.tags.len(), 3);
        for (i, tag) in task.tags.iter().enumerate() {
            assert_eq!(tag.label, format!("Test Tag {}", i + 1));
            assert!(tag.category.is_some());
            if let Some(ref category) = tag.category {
                assert_eq!(category, "Testing");
            }
        }
    }

    #[test]
    async fn select_deleted() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = Utc::now();

        let mut seen_ids = HashSet::new();
        for _ in 0..10 {
            let task = create_test_task(
                &mut tx,
                TaskCreate::default(),
                None,
                Some(base_time + Duration::from_hours(1)),
                base_time,
                base_time + Duration::from_hours(1),
            )
            .await;

            assert!(seen_ids.insert(task.id), "duplicate task encountered");
        }
        assert!(!seen_ids.is_empty());

        for id in seen_ids {
            let res = select_task_inner(&mut tx, id, Uuid::nil()).await;
            assert!(res.is_ok());

            let task_opt = res.unwrap();
            assert!(task_opt.is_none());
        }
    }

    #[test]
    async fn select_nonexistent() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let res = select_task_inner(&mut tx, Uuid::new_v4(), Uuid::nil()).await;
        assert!(res.is_ok());

        let task_opt = res.unwrap();
        assert!(task_opt.is_none());
    }
}

#[cfg(test)]
mod insert_tests {
    use chrono::{Duration, Local, NaiveDate, NaiveDateTime, NaiveTime, SubsecRound, Utc};
    use tokio::test;
    use uuid::Uuid;

    use super::{
        insert_task_inner,
        test_helpers::{PG_SUBSEC_PREC, create_test_tag, get_pool},
    };
    use crate::routes::models::{Start, tag::Model as TagCreate, task::Model as TaskCreate};

    #[test]
    async fn base_insert() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let res = insert_task_inner(&mut tx, Uuid::nil(), TaskCreate::default()).await;
        assert!(res.is_ok());

        let task = res.unwrap();
        assert_eq!(task.title, "");
        assert!(task.notes.is_none());
        assert!(task.start_dt.is_none());
        assert!(task.deadline.is_none());
        assert!(task.tags.is_empty());

        assert!(task.deleted_at.is_none());
        assert_eq!(task.created_by, Uuid::nil());

        assert_eq!(
            task.created_at, task.updated_at,
            "created_at and updated_at should be the same when created"
        );
    }

    #[test]
    async fn with_title() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let title = "This is a test title for with_title() test".to_string();

        let res = insert_task_inner(
            &mut tx,
            Uuid::nil(),
            TaskCreate {
                title: title.clone(),
                ..Default::default()
            },
        )
        .await;
        assert!(res.is_ok());

        let task = res.unwrap();
        assert_eq!(task.title, "This is a test title for with_title() test");
        assert!(task.notes.is_none());
        assert!(task.start_dt.is_none());
        assert!(task.deadline.is_none());
        assert!(task.tags.is_empty());

        assert!(task.deleted_at.is_none());
        assert_eq!(task.created_by, Uuid::nil());

        assert_eq!(
            task.created_at, task.updated_at,
            "created_at and updated_at should be the same when created"
        );
    }

    #[test]
    async fn with_notes() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let notes = "This is the notes section in the with_notes() test".to_string();

        let res = insert_task_inner(
            &mut tx,
            Uuid::nil(),
            TaskCreate {
                notes: Some(notes),
                ..Default::default()
            },
        )
        .await;
        assert!(res.is_ok());

        let task = res.unwrap();
        assert_eq!(task.title, "");
        assert!(task.notes.is_some());
        assert_eq!(
            task.notes.unwrap(),
            "This is the notes section in the with_notes() test"
        );
        assert!(task.start_dt.is_none());
        assert!(task.deadline.is_none());
        assert!(task.tags.is_empty());

        assert!(task.deleted_at.is_none());
        assert_eq!(task.created_by, Uuid::nil());

        assert_eq!(
            task.created_at, task.updated_at,
            "created_at and updated_at should be the same when created"
        );
    }

    #[test]
    async fn with_start_date() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let date = Utc::now().date_naive();

        let res = insert_task_inner(
            &mut tx,
            Uuid::nil(),
            TaskCreate {
                start: Some(Start::On(date)),
                ..Default::default()
            },
        )
        .await;
        assert!(res.is_ok());

        let task = res.unwrap();
        assert_eq!(task.title, "");
        assert!(task.notes.is_none());
        assert!(task.start_dt.is_some());
        assert!(!task.has_time);
        assert!(task.deadline.is_none());
        assert!(task.tags.is_empty());

        assert!(task.deleted_at.is_none());
        assert_eq!(task.created_by, Uuid::nil());

        assert_eq!(
            task.created_at, task.updated_at,
            "created_at and updated_at should be the same when created"
        );
    }

    #[test]
    async fn with_start_datetime() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let datetime = Utc::now();

        let res = insert_task_inner(
            &mut tx,
            Uuid::nil(),
            TaskCreate {
                start: Some(Start::At(datetime)),
                ..Default::default()
            },
        )
        .await;
        assert!(res.is_ok());

        let task = res.unwrap();
        assert_eq!(task.title, "");
        assert!(task.notes.is_none());
        assert!(task.start_dt.is_some());
        assert!(task.has_time);
        assert!(task.deadline.is_none());
        assert!(task.tags.is_empty());

        assert!(task.deleted_at.is_none());
        assert_eq!(task.created_by, Uuid::nil());

        assert_eq!(
            task.created_at, task.updated_at,
            "created_at and updated_at should be the same when created"
        );
    }

    #[test]
    async fn with_deadline() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let date = Utc::now().date_naive();

        let res = insert_task_inner(
            &mut tx,
            Uuid::nil(),
            TaskCreate {
                deadline: Some(date),
                ..Default::default()
            },
        )
        .await;
        assert!(res.is_ok());

        let task = res.unwrap();
        assert_eq!(task.title, "");
        assert!(task.notes.is_none());
        assert!(task.start_dt.is_none());
        assert!(task.deadline.is_some());
        assert_eq!(task.deadline.unwrap(), date);
        assert!(task.tags.is_empty());

        assert!(task.deleted_at.is_none());
        assert_eq!(task.created_by, Uuid::nil());

        assert_eq!(
            task.created_at, task.updated_at,
            "created_at and updated_at should be the same when created"
        );
    }

    #[test]
    async fn with_tag() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = Utc::now();

        let tag = create_test_tag(
            &mut tx,
            TagCreate {
                label: "Test Tag".to_string(),
                category: Some("Testing".to_string()),
            },
            base_time,
            base_time,
        )
        .await;

        let res = insert_task_inner(
            &mut tx,
            Uuid::nil(),
            TaskCreate {
                tags: vec![tag.id],
                ..Default::default()
            },
        )
        .await;
        assert!(res.is_ok());

        let task = res.unwrap();
        assert_eq!(task.title, "");
        assert!(task.notes.is_none());
        assert!(task.start_dt.is_none());
        assert!(task.deadline.is_none());

        assert_eq!(task.tags.len(), 1);
        let tag = task.tags.first().unwrap();
        assert_eq!(tag.label, "Test Tag");
        assert!(tag.category.is_some());
        if let Some(ref category) = tag.category {
            assert_eq!(category, "Testing");
        }

        assert!(task.deleted_at.is_none());
        assert_eq!(task.created_by, Uuid::nil());

        assert_eq!(
            task.created_at, task.updated_at,
            "created_at and updated_at should be the same when created"
        );
    }

    #[test]
    async fn with_tags() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = Utc::now();

        let tag_1 = create_test_tag(
            &mut tx,
            TagCreate {
                label: "Test Tag 1".to_string(),
                category: Some("Testing".to_string()),
            },
            base_time,
            base_time,
        )
        .await;
        let tag_2 = create_test_tag(
            &mut tx,
            TagCreate {
                label: "Test Tag 2".to_string(),
                category: Some("Testing".to_string()),
            },
            base_time,
            base_time,
        )
        .await;
        let tag_3 = create_test_tag(
            &mut tx,
            TagCreate {
                label: "Test Tag 3".to_string(),
                category: Some("Testing".to_string()),
            },
            base_time,
            base_time,
        )
        .await;

        let res = insert_task_inner(
            &mut tx,
            Uuid::nil(),
            TaskCreate {
                title: "Test Task".to_string(),
                tags: vec![tag_1.id, tag_2.id, tag_3.id],
                ..Default::default()
            },
        )
        .await;
        assert!(res.is_ok());

        let task = res.unwrap();
        assert_eq!(task.title, "Test Task");
        assert!(task.notes.is_none());
        assert!(task.start_dt.is_none());
        assert!(task.deadline.is_none());
        assert_eq!(task.tags.len(), 3);
        for (i, tag) in task.tags.iter().enumerate() {
            assert_eq!(tag.label, format!("Test Tag {}", i + 1));
            assert!(tag.category.is_some());
            if let Some(ref category) = tag.category {
                assert_eq!(category, "Testing");
            }
        }

        assert!(task.deleted_at.is_none());
        assert_eq!(task.created_by, Uuid::nil());

        assert_eq!(
            task.created_at, task.updated_at,
            "created_at and updated_at should be the same when created"
        );
    }

    #[test]
    async fn with_nonexistent_tag() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let res = insert_task_inner(
            &mut tx,
            Uuid::nil(),
            TaskCreate {
                tags: vec![Uuid::new_v4()],
                ..Default::default()
            },
        )
        .await;

        assert!(res.is_err());
    }

    #[test]
    async fn combination_1() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let title = "Homework 1".to_string();
        let notes =
            "Introduction assignment to warm up to the content being taught in class".to_string();
        let start = NaiveDate::from_ymd_opt(2027, 9, 16).unwrap();
        let deadline = start + Duration::weeks(2);

        let res = insert_task_inner(
            &mut tx,
            Uuid::nil(),
            TaskCreate {
                title: title.clone(),
                notes: Some(notes.clone()),
                start: Some(Start::On(start)),
                deadline: Some(deadline),
                ..Default::default()
            },
        )
        .await;
        assert!(res.is_ok());

        let task = res.unwrap();
        assert_eq!(task.title, "Homework 1");
        assert!(task.notes.is_some());
        assert_eq!(task.notes.unwrap(), notes);
        assert!(task.start_dt.is_some());
        assert!(!task.has_time);
        assert_eq!(task.start_dt.unwrap().date_naive(), start);
        assert!(task.deadline.is_some());
        assert_eq!(task.deadline.unwrap(), deadline);
        assert!(task.tags.is_empty());

        assert!(task.deleted_at.is_none());
        assert_eq!(task.created_by, Uuid::nil());

        assert_eq!(
            task.created_at, task.updated_at,
            "created_at and updated_at should be the same when created"
        );
    }

    #[test]
    async fn combination_2() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let title = "Study for Exam 1".to_string();
        let notes =
            "Introduction assignment to warm up to the content being taught in class".to_string();
        let start_date = NaiveDate::from_ymd_opt(2027, 10, 5).unwrap();
        let start_time = NaiveTime::from_hms_opt(10, 0, 0).unwrap();
        let start_datetime = NaiveDateTime::new(start_date, start_time)
            .and_local_timezone(*Local::now().offset())
            .unwrap()
            .to_utc();
        let deadline = start_date + Duration::days(4);

        let res = insert_task_inner(
            &mut tx,
            Uuid::nil(),
            TaskCreate {
                title: title.clone(),
                notes: Some(notes.clone()),
                start: Some(Start::At(start_datetime)),
                deadline: Some(deadline),
                ..Default::default()
            },
        )
        .await;
        assert!(res.is_ok());

        let task = res.unwrap();
        assert_eq!(task.title, "Study for Exam 1");
        assert!(task.notes.is_some());
        assert_eq!(task.notes.unwrap(), notes);
        assert!(task.start_dt.is_some());
        assert!(task.has_time);
        assert_eq!(
            task.start_dt.unwrap(),
            start_datetime.trunc_subsecs(PG_SUBSEC_PREC)
        );
        assert!(task.deadline.is_some());
        assert_eq!(task.deadline.unwrap(), deadline);
        assert!(task.tags.is_empty());

        assert!(task.deleted_at.is_none());
        assert_eq!(task.created_by, Uuid::nil());

        assert_eq!(
            task.created_at, task.updated_at,
            "created_at and updated_at should be the same when created"
        );
    }
}

#[cfg(test)]
mod update_tests {
    use chrono::{Duration, NaiveDate, SubsecRound, Utc};
    use tokio::test;
    use uuid::Uuid;

    use super::{
        test_helpers::{PG_SUBSEC_PREC, create_test_tag, create_test_task, get_pool, get_task},
        update_task_inner,
    };
    use crate::{
        db::{ApplicationError, Error},
        routes::models::{Start, tag::Model as TagCreate, task::Model as TaskCreate},
    };

    #[test]
    async fn only_changes_correct_fields() {
        // TODO: check no changes to
        // - id
        // - completed_at
        // - deleted_at
        // - created_at
        // - created_by

        // Test with changes to:
        // - title only
        // - notes only
        // - start (none to on, none to at, from on to at, from at to on, on to none, at to none)
        // - deadline only
        // - tag only
    }

    #[test]
    async fn title_only() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = Utc::now();

        let before_task = create_test_task(
            &mut tx,
            TaskCreate::default(),
            None,
            None,
            base_time,
            base_time,
        )
        .await;

        let mut task_update: TaskCreate = before_task.clone().into();
        task_update.title = "New title".to_string();

        let res = update_task_inner(&mut tx, before_task.id, Uuid::nil(), task_update).await;
        assert!(res.is_ok());

        let after_task = res.unwrap();
        assert_ne!(before_task.updated_at, after_task.updated_at);
        assert_ne!(before_task.title, after_task.title);
        assert_eq!(after_task.title, "New title");

        assert_eq!(before_task.notes, after_task.notes);
        assert_eq!(before_task.start_dt, after_task.start_dt);
        assert_eq!(before_task.has_time, after_task.has_time);
        assert_eq!(before_task.deadline, after_task.deadline);
        assert_eq!(before_task.tags, after_task.tags);
    }

    #[test]
    async fn notes_only() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = Utc::now();

        let before_task = create_test_task(
            &mut tx,
            TaskCreate::default(),
            None,
            None,
            base_time,
            base_time,
        )
        .await;

        let mut updated_task: TaskCreate = before_task.clone().into();
        updated_task.notes = Some("Updated notes".to_string());

        let res = update_task_inner(&mut tx, before_task.id, Uuid::nil(), updated_task).await;
        assert!(res.is_ok());

        let after_task = res.unwrap();
        assert_ne!(before_task.updated_at, after_task.updated_at);
        assert_ne!(before_task.notes, after_task.notes);
        assert!(after_task.notes.is_some());
        if let Some(notes) = after_task.notes {
            assert_eq!(notes, "Updated notes");
        }

        assert_eq!(before_task.title, after_task.title);
        assert_eq!(before_task.start_dt, after_task.start_dt);
        assert_eq!(before_task.has_time, after_task.has_time);
        assert_eq!(before_task.deadline, after_task.deadline);
        assert_eq!(before_task.tags, after_task.tags);
    }

    #[test]
    async fn start_on_only() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = Utc::now();

        let before_task = create_test_task(
            &mut tx,
            TaskCreate::default(),
            None,
            None,
            base_time,
            base_time,
        )
        .await;

        let updated_start = Utc::now().date_naive();
        let mut updated_task: TaskCreate = before_task.clone().into();
        updated_task.start = Some(Start::On(updated_start));

        let res = update_task_inner(&mut tx, before_task.id, Uuid::nil(), updated_task).await;
        assert!(res.is_ok());

        let after_task = res.unwrap();
        assert_ne!(before_task.updated_at, after_task.updated_at);
        assert_ne!(before_task.start_dt, after_task.start_dt);
        assert_eq!(after_task.start_dt.unwrap().date_naive(), updated_start);
        assert_eq!(before_task.title, after_task.title);
        assert_eq!(before_task.notes, after_task.notes);
        assert_eq!(before_task.has_time, after_task.has_time);
        assert_eq!(before_task.deadline, after_task.deadline);
        assert_eq!(before_task.tags, after_task.tags);

        // TODO: test at to on
    }

    #[test]
    async fn start_at_only() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = Utc::now();

        let before_task = create_test_task(
            &mut tx,
            TaskCreate::default(),
            None,
            None,
            base_time,
            base_time,
        )
        .await;

        let updated_start = Utc::now();
        let mut updated_task: TaskCreate = before_task.clone().into();
        updated_task.start = Some(Start::At(updated_start));

        let res = update_task_inner(&mut tx, before_task.id, Uuid::nil(), updated_task).await;
        assert!(res.is_ok());

        let after_task = res.unwrap();
        assert_ne!(before_task.updated_at, after_task.updated_at);
        assert_ne!(before_task.start_dt, after_task.start_dt);
        assert_ne!(before_task.has_time, after_task.has_time);
        assert_eq!(
            after_task.start_dt.unwrap(),
            updated_start.trunc_subsecs(PG_SUBSEC_PREC)
        );

        assert_eq!(before_task.title, after_task.title);
        assert_eq!(before_task.notes, after_task.notes);
        assert_eq!(before_task.deadline, after_task.deadline);
        assert_eq!(before_task.tags, after_task.tags);

        // TODO: test on to at
    }

    #[test]
    async fn start_to_none() {
        // TODO: update this function
        // test conversions:
        // * on to none
        // * at to none

        // let pool = get_pool().await;
        // let mut tx = pool.begin().await.unwrap();

        // let base_time = Utc::now();

        // let datetime = Utc::now();
        // let date = datetime.date_naive();

        // let before_task = create_test_task(
        //     &mut tx,
        //     TaskCreate {
        //         start: Some(Start::On(date)),
        //         ..Default::default()
        //     },
        //     None,
        //     None,
        //     base_time,
        //     base_time,
        // )
        // .await;

        // let mut updated_task: TaskCreate = before_task.clone().into();
        // updated_task.start = Some(Start::At(datetime));

        // let res = update_task_inner(&mut tx, before_task.id, Uuid::nil(), updated_task).await;
        // assert!(res.is_ok());

        // let after_task = res.unwrap();
        // assert_ne!(before_task.updated_at, after_task.updated_at);
        // assert_ne!(before_task.start_on, after_task.start_on);
        // assert!(after_task.start_on.is_none());
        // assert_ne!(before_task.start_at, after_task.start_at);
        // assert!(after_task.start_at.is_some());
        // assert_eq!(
        //     after_task.start_at.unwrap(),
        //     datetime.trunc_subsecs(PG_SUBSEC_PREC)
        // );

        // assert_eq!(before_task.title, after_task.title);
        // assert_eq!(before_task.notes, after_task.notes);
        // assert_eq!(before_task.deadline, after_task.deadline);
        // assert_eq!(before_task.tags, after_task.tags);
    }

    #[test]
    async fn deadline_only() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = Utc::now();

        let before_task = create_test_task(
            &mut tx,
            TaskCreate::default(),
            None,
            None,
            base_time,
            base_time,
        )
        .await;

        let updated_deadline = Utc::now().date_naive();
        let mut updated_task: TaskCreate = before_task.clone().into();
        updated_task.deadline = Some(updated_deadline);

        let res = update_task_inner(&mut tx, before_task.id, Uuid::nil(), updated_task).await;
        assert!(res.is_ok());

        let after_task = res.unwrap();
        assert_ne!(before_task.updated_at, after_task.updated_at);
        assert_ne!(before_task.deadline, after_task.deadline);
        assert!(after_task.deadline.is_some());
        if let Some(date) = after_task.deadline {
            assert_eq!(date, updated_deadline);
        }

        assert_eq!(before_task.title, after_task.title);
        assert_eq!(before_task.notes, after_task.notes);
        assert_eq!(before_task.start_dt, after_task.start_dt);
        assert_eq!(before_task.tags, after_task.tags);
    }

    #[test]
    async fn update_tag() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = Utc::now();

        let tag = create_test_tag(
            &mut tx,
            TagCreate {
                label: "Test Tag".to_string(),
                category: Some("Testing".to_string()),
            },
            base_time,
            base_time,
        )
        .await;

        let before_task = create_test_task(
            &mut tx,
            TaskCreate::default(),
            None,
            None,
            base_time,
            base_time,
        )
        .await;

        let mut updated_task: TaskCreate = before_task.clone().into();
        updated_task.tags = vec![tag.id];

        let res = update_task_inner(&mut tx, before_task.id, Uuid::nil(), updated_task).await;
        assert!(res.is_ok());

        let after_task = res.unwrap();
        assert_ne!(before_task.updated_at, after_task.updated_at);
        assert_ne!(before_task.tags, after_task.tags);

        assert_eq!(after_task.tags.len(), 1);
        let tag = after_task.tags.first().unwrap();
        assert_eq!(tag.label, "Test Tag");
        assert!(tag.category.is_some());
        if let Some(ref category) = tag.category {
            assert_eq!(category, "Testing");
        }

        assert_eq!(before_task.title, after_task.title);
        assert_eq!(before_task.notes, after_task.notes);
        assert_eq!(before_task.start_dt, after_task.start_dt);
        assert_eq!(before_task.deadline, after_task.deadline);
    }

    #[test]
    async fn update_tags() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = Utc::now();

        let tag_1 = create_test_tag(
            &mut tx,
            TagCreate {
                label: "Test Tag 1".to_string(),
                category: Some("Testing".to_string()),
            },
            base_time,
            base_time,
        )
        .await;
        let tag_2 = create_test_tag(
            &mut tx,
            TagCreate {
                label: "Test Tag 2".to_string(),
                category: Some("Testing".to_string()),
            },
            base_time,
            base_time,
        )
        .await;
        let tag_3 = create_test_tag(
            &mut tx,
            TagCreate {
                label: "Test Tag 3".to_string(),
                category: Some("Testing".to_string()),
            },
            base_time,
            base_time,
        )
        .await;

        let before_task = create_test_task(
            &mut tx,
            TaskCreate::default(),
            None,
            None,
            base_time,
            base_time,
        )
        .await;

        let mut updated_task: TaskCreate = before_task.clone().into();
        updated_task.tags = vec![tag_1.id, tag_2.id, tag_3.id];

        let res = update_task_inner(&mut tx, before_task.id, Uuid::nil(), updated_task).await;
        assert!(res.is_ok());

        let after_task = res.unwrap();
        assert_ne!(before_task.updated_at, after_task.updated_at);
        assert_ne!(before_task.tags, after_task.tags);
        assert_eq!(after_task.tags.len(), 3);
        for (i, tag) in after_task.tags.iter().enumerate() {
            assert_eq!(tag.label, format!("Test Tag {}", i + 1));
            assert!(tag.category.is_some());
            if let Some(ref category) = tag.category {
                assert_eq!(category, "Testing");
            }
        }

        assert_eq!(before_task.title, after_task.title);
        assert_eq!(before_task.notes, after_task.notes);
        assert_eq!(before_task.start_dt, after_task.start_dt);
        assert_eq!(before_task.deadline, after_task.deadline);
    }

    #[test]
    async fn update_empty_tag() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = Utc::now();

        let tag_1 = create_test_tag(
            &mut tx,
            TagCreate {
                label: "Test Tag 1".to_string(),
                category: Some("Testing".to_string()),
            },
            base_time,
            base_time,
        )
        .await;
        let tag_2 = create_test_tag(
            &mut tx,
            TagCreate {
                label: "Test Tag 2".to_string(),
                category: Some("Testing".to_string()),
            },
            base_time,
            base_time,
        )
        .await;
        let tag_3 = create_test_tag(
            &mut tx,
            TagCreate {
                label: "Test Tag 3".to_string(),
                category: Some("Testing".to_string()),
            },
            base_time,
            base_time,
        )
        .await;

        let before_task = create_test_task(
            &mut tx,
            TaskCreate {
                tags: vec![tag_1.id, tag_2.id, tag_3.id],
                ..Default::default()
            },
            None,
            None,
            base_time,
            base_time,
        )
        .await;

        let mut updated_task: TaskCreate = before_task.clone().into();
        updated_task.tags = vec![];

        let res = update_task_inner(&mut tx, before_task.id, Uuid::nil(), updated_task).await;
        assert!(res.is_ok());

        let after_task = res.unwrap();
        assert_ne!(before_task.updated_at, after_task.updated_at);
        assert_ne!(before_task.tags, after_task.tags);
        assert!(after_task.tags.is_empty());

        assert_eq!(before_task.title, after_task.title);
        assert_eq!(before_task.notes, after_task.notes);
        assert_eq!(before_task.start_dt, after_task.start_dt);
        assert_eq!(before_task.deadline, after_task.deadline);
    }

    #[test]
    async fn update_nonexistent_tag() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = Utc::now();

        let before_task = create_test_task(
            &mut tx,
            TaskCreate::default(),
            None,
            None,
            base_time,
            base_time,
        )
        .await;

        let mut updated_task: TaskCreate = before_task.clone().into();
        updated_task.tags = vec![Uuid::new_v4()];

        let res = update_task_inner(&mut tx, before_task.id, Uuid::nil(), updated_task).await;
        assert!(matches!(
            res,
            Err(Error::Application(ApplicationError::TagNotFound))
        ));
    }

    #[test]
    async fn deleted_task() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = Utc::now();

        let task = create_test_task(
            &mut tx,
            TaskCreate::default(),
            None,
            Some(base_time),
            base_time,
            base_time,
        )
        .await;

        let res = update_task_inner(&mut tx, task.id, Uuid::nil(), TaskCreate::default()).await;
        assert!(matches!(
            res,
            Err(Error::Application(ApplicationError::TaskNotFound))
        ));
    }

    // TODO: uncomment when updates are finally idempotent
    // #[test]
    // async fn is_idempotent() {
    //     let pool = get_pool().await;
    //     let mut tx = pool.begin().await.unwrap();

    //     let base_time = Utc::now();

    //     let before_task = create_test_task(
    //         &mut tx,
    //         TaskCreate::default(),
    //         None,
    //         None,
    //         base_time,
    //         base_time,
    //     )
    //     .await;

    //     let task_update: TaskCreate = before_task.clone().into();

    //     let res = update_task_inner(&mut tx, before_task.id, Uuid::nil(), task_update).await;
    //     assert!(res.is_ok());

    //     let after_task = res.unwrap();
    //     assert_eq!(before_task.id, after_task.id);
    //     assert_eq!(before_task.title, after_task.title);
    //     assert_eq!(before_task.notes, after_task.notes);
    //     assert_eq!(before_task.start_on, after_task.start_on);
    //     assert_eq!(before_task.start_at, after_task.start_at);
    //     assert_eq!(before_task.deadline, after_task.deadline);
    //     assert_eq!(before_task.tags, after_task.tags);
    //     assert_eq!(before_task.completed_at, after_task.completed_at);
    //     assert_eq!(before_task.deleted_at, after_task.deleted_at);
    //     assert_eq!(before_task.created_at, after_task.created_at);
    //     assert_eq!(before_task.updated_at, after_task.updated_at);
    //     assert_eq!(before_task.created_by, after_task.created_by);
    // }

    #[test]
    async fn nonexistent_task() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let res =
            update_task_inner(&mut tx, Uuid::new_v4(), Uuid::nil(), TaskCreate::default()).await;
        assert!(matches!(
            res,
            Err(Error::Application(ApplicationError::TaskNotFound))
        ));
    }

    #[test]
    async fn combination_1() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = Utc::now();

        let before_task = create_test_task(
            &mut tx,
            TaskCreate {
                title: "Homework 2".to_string(),
                notes: Some("Finish problems 1-38 from textbook".to_string()),
                start: Some(Start::On(NaiveDate::from_ymd_opt(2026, 10, 5).unwrap())),
                deadline: Some(NaiveDate::from_ymd_opt(2026, 10, 19).unwrap()),
                ..Default::default()
            },
            None,
            None,
            base_time,
            base_time,
        )
        .await;

        let mut updated_task: TaskCreate = before_task.clone().into();
        if let Some(Start::On(date)) = updated_task.start {
            updated_task.start = Some(Start::On(date + Duration::weeks(1)));
        }
        if let Some(date) = updated_task.deadline {
            updated_task.deadline = Some(date + Duration::weeks(1));
        }

        let res = update_task_inner(&mut tx, before_task.id, Uuid::nil(), updated_task).await;
        assert!(res.is_ok());

        let after_task = res.unwrap();
        assert_ne!(before_task.updated_at, after_task.updated_at);
        assert_ne!(before_task.start_dt, after_task.start_dt);
        if let (Some(before_date), Some(after_date)) = (before_task.start_dt, after_task.start_dt) {
            assert_eq!(before_date + Duration::weeks(1), after_date);
        }
        assert_ne!(before_task.deadline, after_task.deadline);
        if let (Some(before_date), Some(after_date)) = (before_task.deadline, after_task.deadline) {
            assert_eq!(before_date + Duration::weeks(1), after_date);
        }

        assert_eq!(before_task.title, after_task.title);
        assert_eq!(before_task.notes, after_task.notes);
        assert_eq!(before_task.tags, after_task.tags);
    }

    #[test]
    async fn combination_2() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = Utc::now();

        let backlog_tag = create_test_tag(
            &mut tx,
            TagCreate {
                label: "Backlog".to_string(),
                category: Some("Workflow".to_string()),
            },
            base_time,
            base_time,
        )
        .await;
        let todo_tag = create_test_tag(
            &mut tx,
            TagCreate {
                label: "Todo".to_string(),
                category: Some("Workflow".to_string()),
            },
            base_time,
            base_time,
        )
        .await;
        let in_progress_tag = create_test_tag(
            &mut tx,
            TagCreate {
                label: "In Progress".to_string(),
                category: Some("Workflow".to_string()),
            },
            base_time,
            base_time,
        )
        .await;
        let completed_tag = create_test_tag(
            &mut tx,
            TagCreate {
                label: "Completed".to_string(),
                category: Some("Workflow".to_string()),
            },
            base_time,
            base_time,
        )
        .await;

        let task = create_test_task(
            &mut tx,
            TaskCreate {
                title: "Create reusable button component".to_string(),
                tags: vec![backlog_tag.id],
                ..Default::default()
            },
            None,
            None,
            base_time,
            base_time,
        )
        .await;

        let backlog_task = get_task(&mut tx, task.id).await;

        assert_eq!(backlog_task.tags.len(), 1);
        let tag = backlog_task.tags.first().unwrap();
        assert_eq!(tag.label, "Backlog");
        assert!(tag.category.is_some());
        if let Some(ref category) = tag.category {
            assert_eq!(category, "Workflow");
        }

        let mut updated_task: TaskCreate = backlog_task.clone().into();
        updated_task.tags = vec![todo_tag.id];

        let res = update_task_inner(&mut tx, task.id, Uuid::nil(), updated_task).await;
        assert!(res.is_ok());

        let todo_task = res.unwrap();

        assert_eq!(todo_task.tags.len(), 1);
        let tag = todo_task.tags.first().unwrap();
        assert_eq!(tag.label, "Todo");
        assert!(tag.category.is_some());
        if let Some(ref category) = tag.category {
            assert_eq!(category, "Workflow");
        }

        let mut updated_task: TaskCreate = backlog_task.clone().into();
        updated_task.tags = vec![in_progress_tag.id];

        let res = update_task_inner(&mut tx, task.id, Uuid::nil(), updated_task).await;
        assert!(res.is_ok());

        let in_progress_task = res.unwrap();

        assert_eq!(in_progress_task.tags.len(), 1);
        let tag = in_progress_task.tags.first().unwrap();
        assert_eq!(tag.label, "In Progress");
        assert!(tag.category.is_some());
        if let Some(ref category) = tag.category {
            assert_eq!(category, "Workflow");
        }

        let mut updated_task: TaskCreate = backlog_task.clone().into();
        updated_task.tags = vec![completed_tag.id];

        let res = update_task_inner(&mut tx, task.id, Uuid::nil(), updated_task).await;
        assert!(res.is_ok());

        let completed_task = res.unwrap();

        assert_eq!(completed_task.tags.len(), 1);
        let tag = completed_task.tags.first().unwrap();
        assert_eq!(tag.label, "Completed");
        assert!(tag.category.is_some());
        if let Some(ref category) = tag.category {
            assert_eq!(category, "Workflow");
        }
    }

    #[test]
    async fn combination_3() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = Utc::now();

        let before_task = create_test_task(
            &mut tx,
            TaskCreate {
                title: "Create database schema".to_string(),
                notes: Some("Database schema for todo list".to_string()),
                ..Default::default()
            },
            None,
            None,
            base_time,
            base_time,
        )
        .await;

        let mut updated_task: TaskCreate = before_task.clone().into();
        updated_task.title = "Create task database schema".to_string();
        updated_task.notes =
            Some("Schema should contain:\n* id\n* title\n* notes\n* created_by".to_string());

        let res = update_task_inner(&mut tx, before_task.id, Uuid::nil(), updated_task).await;
        assert!(res.is_ok());

        let after_task = res.unwrap();
        assert_ne!(before_task.updated_at, after_task.updated_at);
        assert_ne!(before_task.title, after_task.title);
        assert_eq!(after_task.title, "Create task database schema");
        assert_ne!(before_task.notes, after_task.notes);
        assert!(after_task.notes.is_some());
        if let Some(notes) = after_task.notes {
            assert_eq!(
                notes,
                "Schema should contain:\n* id\n* title\n* notes\n* created_by"
            );
        }
    }
}

#[cfg(test)]
mod delete_tests {
    use chrono::Utc;
    use tokio::test;
    use uuid::Uuid;

    use super::{
        delete_task_inner,
        test_helpers::{create_test_task, get_pool, get_task},
    };
    use crate::routes::models::task::Model as TaskCreate;

    #[test]
    async fn base_delete() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = Utc::now();

        let task = create_test_task(
            &mut tx,
            TaskCreate::default(),
            None,
            None,
            base_time,
            base_time,
        )
        .await;

        let res = delete_task_inner(&mut tx, task.id, Uuid::nil()).await;
        assert!(res.is_ok());

        assert_eq!(res.unwrap(), (), "delete should return unit '()'");

        let after_task = get_task(&mut tx, task.id).await;
        assert_ne!(after_task.updated_at, task.updated_at);
        assert!(after_task.deleted_at.is_some());
        if let Some(date) = after_task.deleted_at {
            assert_eq!(date, after_task.updated_at);
        }
    }

    #[test]
    async fn only_changes_correct_fields() {
        // TODO: make sure that delete only updates deleted_at and updated_at
    }

    #[test]
    async fn deleted_task() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = Utc::now();

        let task = create_test_task(
            &mut tx,
            TaskCreate::default(),
            None,
            Some(base_time),
            base_time,
            base_time,
        )
        .await;

        let res = delete_task_inner(&mut tx, task.id, Uuid::nil()).await;
        assert!(res.is_ok());

        assert_eq!(res.unwrap(), (), "deleted should return unit '()'");

        let after_task = get_task(&mut tx, task.id).await;
        assert_eq!(
            after_task.updated_at, task.updated_at,
            "updated_at should not be updated if the task was already deleted"
        );
        assert_eq!(
            after_task.deleted_at, task.deleted_at,
            "deleted_at should not be updated if the task was already deleted"
        );
    }

    #[test]
    async fn is_idempotent() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = Utc::now();

        let task = create_test_task(
            &mut tx,
            TaskCreate::default(),
            None,
            None,
            base_time,
            base_time,
        )
        .await;

        let res = delete_task_inner(&mut tx, task.id, Uuid::nil()).await;
        assert!(res.is_ok());

        assert_eq!(res.unwrap(), (), "delete should return unit '()'");

        let first_delete_task = get_task(&mut tx, task.id).await;
        assert_ne!(first_delete_task.updated_at, task.updated_at);
        assert!(first_delete_task.deleted_at.is_some());
        if let Some(date) = first_delete_task.deleted_at {
            assert_eq!(date, first_delete_task.updated_at);
        }

        let res = delete_task_inner(&mut tx, task.id, Uuid::nil()).await;
        assert!(res.is_ok());

        assert_eq!(res.unwrap(), (), "deleted should return unit '()'");

        let second_delete_task = get_task(&mut tx, task.id).await;
        assert_eq!(
            second_delete_task.updated_at, first_delete_task.updated_at,
            "updated_at should not be updated if the task was already deleted"
        );
        assert_eq!(
            second_delete_task.deleted_at, first_delete_task.deleted_at,
            "deleted_at should not be updated if the task was already deleted"
        );
    }

    #[test]
    async fn nonexistent_task() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let res = delete_task_inner(&mut tx, Uuid::new_v4(), Uuid::nil()).await;
        assert!(res.is_ok());

        assert_eq!(res.unwrap(), (), "delete should return unit '()'");
    }
}

#[cfg(test)]
mod restore_tests {
    use chrono::Utc;
    use tokio::test;
    use uuid::Uuid;

    use super::{
        restore_task_inner,
        test_helpers::{create_test_task, get_pool, get_task},
    };
    use crate::{
        db::{ApplicationError, Error},
        routes::models::task::Model as TaskCreate,
    };

    #[test]
    async fn base_restore() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = Utc::now();

        let task = create_test_task(
            &mut tx,
            TaskCreate::default(),
            None,
            Some(base_time),
            base_time,
            base_time,
        )
        .await;

        let res = restore_task_inner(&mut tx, task.id, Uuid::nil()).await;
        assert!(res.is_ok());

        assert_eq!(res.unwrap(), (), "restore should return unit '()'");

        let after_task = get_task(&mut tx, task.id).await;
        assert_ne!(after_task.updated_at, task.updated_at);
        assert!(after_task.deleted_at.is_none());
    }

    #[test]
    async fn only_changes_correct_fields() {
        // TODO: make sure that delete only updates deleted_at and updated_at
    }

    #[test]
    async fn restored_task() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = Utc::now();

        let task = create_test_task(
            &mut tx,
            TaskCreate::default(),
            None,
            None,
            base_time,
            base_time,
        )
        .await;

        let res = restore_task_inner(&mut tx, task.id, Uuid::nil()).await;
        assert!(res.is_ok());

        assert_eq!(res.unwrap(), (), "restore should return unit '()'");

        let after_task = get_task(&mut tx, task.id).await;
        assert_eq!(
            after_task.updated_at, task.updated_at,
            "updated_at should not be updated if the task was already restored"
        );
        assert_eq!(
            after_task.deleted_at, task.deleted_at,
            "deleted_at should not be updated if the task was already restored"
        );
    }

    #[test]
    async fn is_idempotent() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = Utc::now();

        let task = create_test_task(
            &mut tx,
            TaskCreate::default(),
            None,
            Some(base_time),
            base_time,
            base_time,
        )
        .await;

        let res = restore_task_inner(&mut tx, task.id, Uuid::nil()).await;
        assert!(res.is_ok());

        assert_eq!(res.unwrap(), (), "restore should return unit '()'");

        let first_restore_task = get_task(&mut tx, task.id).await;
        assert_ne!(first_restore_task.updated_at, task.updated_at);
        assert!(first_restore_task.deleted_at.is_none());

        let res = restore_task_inner(&mut tx, task.id, Uuid::nil()).await;
        assert!(res.is_ok());

        assert_eq!(res.unwrap(), (), "restore should return unit '()'");

        let second_restore_task = get_task(&mut tx, task.id).await;
        assert_eq!(
            second_restore_task.updated_at, first_restore_task.updated_at,
            "updated_at should not be updated if the task was already restored"
        );
        assert_eq!(
            second_restore_task.deleted_at, first_restore_task.deleted_at,
            "deleted_at should not be updated if the task was already restore"
        );
    }

    #[test]
    async fn nonexistent_task() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let res = restore_task_inner(&mut tx, Uuid::new_v4(), Uuid::nil()).await;
        assert!(matches!(
            res,
            Err(Error::Application(ApplicationError::TaskNotFound))
        ));
    }
}

#[cfg(test)]
mod complete_tests {
    use chrono::Utc;
    use tokio::test;
    use uuid::Uuid;

    use super::{
        complete_task_inner,
        test_helpers::{create_test_task, get_pool, get_task},
    };
    use crate::{
        db::{ApplicationError, Error},
        routes::models::task::Model as TaskCreate,
    };

    #[test]
    async fn complete_task() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = Utc::now();

        let task = create_test_task(
            &mut tx,
            TaskCreate::default(),
            None,
            None,
            base_time,
            base_time,
        )
        .await;

        let res = complete_task_inner(&mut tx, task.id, Uuid::nil(), true).await;
        assert!(res.is_ok());

        assert_eq!(res.unwrap(), (), "complete should return unit '()'");

        let after_task = get_task(&mut tx, task.id).await;
        assert_ne!(after_task.updated_at, task.updated_at);
        assert!(after_task.completed_at.is_some());
        if let Some(date) = after_task.completed_at {
            assert_eq!(date, after_task.updated_at);
        }
    }

    #[test]
    async fn complete_only_changes_correct_fields() {
        // TODO: make sure that delete only updates completed_at and updated_at
    }

    #[test]
    async fn complete_completed() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = Utc::now();

        let task = create_test_task(
            &mut tx,
            TaskCreate::default(),
            Some(base_time),
            None,
            base_time,
            base_time,
        )
        .await;

        let res = complete_task_inner(&mut tx, task.id, Uuid::nil(), true).await;
        assert!(res.is_ok());

        assert_eq!(res.unwrap(), (), "complete should return unit '()'");

        let after_task = get_task(&mut tx, task.id).await;
        assert_eq!(
            after_task.updated_at, task.updated_at,
            "updated_at should not be updated if task was already completed"
        );
        assert_eq!(
            after_task.completed_at, task.completed_at,
            "completed_at should not be updated if task was already completed"
        );
    }

    #[test]
    async fn complete_is_idempotent() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = Utc::now();

        let task = create_test_task(
            &mut tx,
            TaskCreate::default(),
            None,
            None,
            base_time,
            base_time,
        )
        .await;

        let res = complete_task_inner(&mut tx, task.id, Uuid::nil(), true).await;
        assert!(res.is_ok());

        assert_eq!(res.unwrap(), (), "complete should return unit '()'");

        let first_complete_task = get_task(&mut tx, task.id).await;
        assert_ne!(first_complete_task.updated_at, task.updated_at);
        assert!(first_complete_task.completed_at.is_some());
        if let Some(date) = first_complete_task.completed_at {
            assert_eq!(date, first_complete_task.updated_at);
        }

        let res = complete_task_inner(&mut tx, task.id, Uuid::nil(), true).await;
        assert!(res.is_ok());

        assert_eq!(res.unwrap(), (), "complete should return unit '()'");

        let second_complete_task = get_task(&mut tx, task.id).await;
        assert_eq!(
            second_complete_task.updated_at, first_complete_task.updated_at,
            "updated_at should not be updated if task was already completed"
        );
        assert_eq!(
            second_complete_task.completed_at, first_complete_task.completed_at,
            "completed_at should not be updated if task was already completed"
        );
    }

    #[test]
    async fn complete_deleted() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = Utc::now();

        // uncomplete task
        let task = create_test_task(
            &mut tx,
            TaskCreate::default(),
            None,
            Some(base_time),
            base_time,
            base_time,
        )
        .await;

        let res = complete_task_inner(&mut tx, task.id, Uuid::nil(), true).await;
        assert!(matches!(
            res,
            Err(Error::Application(ApplicationError::TaskNotFound))
        ));

        // completed task
        let task = create_test_task(
            &mut tx,
            TaskCreate::default(),
            Some(base_time),
            Some(base_time),
            base_time,
            base_time,
        )
        .await;

        let res = complete_task_inner(&mut tx, task.id, Uuid::nil(), true).await;
        assert!(matches!(
            res,
            Err(Error::Application(ApplicationError::TaskNotFound))
        ));
    }

    #[test]
    async fn complete_nonexistent() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let res = complete_task_inner(&mut tx, Uuid::new_v4(), Uuid::nil(), true).await;
        assert!(matches!(
            res,
            Err(Error::Application(ApplicationError::TaskNotFound))
        ));
    }

    #[test]
    async fn uncomplete_task() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = Utc::now();

        let task = create_test_task(
            &mut tx,
            TaskCreate::default(),
            Some(base_time),
            None,
            base_time,
            base_time,
        )
        .await;

        let res = complete_task_inner(&mut tx, task.id, Uuid::nil(), false).await;
        assert!(res.is_ok());

        assert_eq!(res.unwrap(), (), "complete should return unit '()'");

        let after_task = get_task(&mut tx, task.id).await;
        assert_ne!(after_task.updated_at, task.updated_at);
        assert!(after_task.completed_at.is_none());
    }

    #[test]
    async fn uncomplete_only_changes_correct_fields() {
        // TODO: make sure that delete only updates completed_at and updated_at
    }

    #[test]
    async fn uncomplete_uncompleted() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = Utc::now();

        let task = create_test_task(
            &mut tx,
            TaskCreate::default(),
            None,
            None,
            base_time,
            base_time,
        )
        .await;

        let res = complete_task_inner(&mut tx, task.id, Uuid::nil(), false).await;
        assert!(res.is_ok());

        assert_eq!(res.unwrap(), (), "complete should return unit '()'");

        let after_task = get_task(&mut tx, task.id).await;
        assert_eq!(
            after_task.updated_at, task.updated_at,
            "updated_at should not be updated if task was already not completed"
        );
        assert_eq!(
            after_task.completed_at, task.completed_at,
            "completed_at should not be updated if task was already not completed"
        );
    }

    #[test]
    async fn uncomplete_is_idempotent() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = Utc::now();

        let task = create_test_task(
            &mut tx,
            TaskCreate::default(),
            Some(base_time),
            None,
            base_time,
            base_time,
        )
        .await;

        let res = complete_task_inner(&mut tx, task.id, Uuid::nil(), false).await;
        assert!(res.is_ok());

        assert_eq!(res.unwrap(), (), "complete should return unit '()'");

        let first_uncomplete_task = get_task(&mut tx, task.id).await;
        assert_ne!(first_uncomplete_task.updated_at, task.updated_at);
        assert!(first_uncomplete_task.completed_at.is_none());

        let res = complete_task_inner(&mut tx, task.id, Uuid::nil(), false).await;
        assert!(res.is_ok());

        assert_eq!(res.unwrap(), (), "complete should return unit '()'");

        let second_uncomplete_task = get_task(&mut tx, task.id).await;
        assert_eq!(
            second_uncomplete_task.updated_at, first_uncomplete_task.updated_at,
            "updated_at should not be updated if task was already not completed"
        );
        assert_eq!(
            second_uncomplete_task.completed_at, first_uncomplete_task.completed_at,
            "completed_at should not be updated if task was already not completed"
        );
    }

    #[test]
    async fn uncomplete_deleted() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = Utc::now();

        // uncomplete task
        let task = create_test_task(
            &mut tx,
            TaskCreate::default(),
            None,
            Some(base_time),
            base_time,
            base_time,
        )
        .await;

        let res = complete_task_inner(&mut tx, task.id, Uuid::nil(), false).await;
        assert!(matches!(
            res,
            Err(Error::Application(ApplicationError::TaskNotFound))
        ));

        // completed task
        let task = create_test_task(
            &mut tx,
            TaskCreate::default(),
            Some(base_time),
            Some(base_time),
            base_time,
            base_time,
        )
        .await;

        let res = complete_task_inner(&mut tx, task.id, Uuid::nil(), false).await;
        assert!(matches!(
            res,
            Err(Error::Application(ApplicationError::TaskNotFound))
        ));
    }

    #[test]
    async fn uncomplete_nonexistent() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let res = complete_task_inner(&mut tx, Uuid::new_v4(), Uuid::nil(), false).await;
        assert!(matches!(
            res,
            Err(Error::Application(ApplicationError::TaskNotFound))
        ));
    }
}
