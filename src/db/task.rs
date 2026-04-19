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
    db::utils::sort_task_tag,
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
        TaskSort::Created(SortOrder::Ascending) => builder.push(" ORDER BY created_at ASC, id ASC"),
        TaskSort::Created(SortOrder::Descending) => {
            builder.push(" ORDER BY created_at DESC, id ASC")
        }
        TaskSort::Updated(SortOrder::Ascending) => builder.push(" ORDER BY updated_at ASC, id ASC"),
        TaskSort::Updated(SortOrder::Descending) => {
            builder.push(" ORDER BY updated_at DESC, id ASC")
        }
        TaskSort::Title(SortOrder::Ascending) => {
            builder.push(" ORDER BY LOWER(title) ASC, updated_at DESC, id ASC")
        }
        TaskSort::Title(SortOrder::Descending) => {
            builder.push(" ORDER BY LOWER(title) DESC, updated_at DESC, id ASC")
        }
        TaskSort::Start(SortOrder::Ascending) => {
            builder.push(" ORDER BY start_dt ASC NULLS LAST, updated_at DESC, id ASC")
        }
        TaskSort::Start(SortOrder::Descending) => {
            builder.push(" ORDER BY start_dt DESC NULLS LAST, updated_at DESC, id ASC")
        }
        TaskSort::Deadline(SortOrder::Ascending) => {
            builder.push(" ORDER BY deadline ASC NULLS LAST, updated_at DESC, id ASC")
        }
        TaskSort::Deadline(SortOrder::Descending) => {
            builder.push(" ORDER BY deadline DESC NULLS LAST, updated_at DESC, id ASC")
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
            task.tags.sort_by(sort_task_tag);
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
    let res = query_as_wrapper::<Task>(
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
    .await;

    let mut task_row = match res {
        Ok(row) => row,
        Err(err) => {
            if let Some(db_err) = err.as_database_error()
                && let Some("tasks_created_by_fkey") = db_err.constraint()
            {
                return Err(Error::Application(ApplicationError::UserNotFound));
            }

            return Err(err.into());
        }
    };

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
    let task_opt = query_as_wrapper::<Task>(
        "UPDATE app.tasks
        SET (title, notes, start_dt, has_time, deadline)
        = ($3, $4, $5, $6, $7)
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
mod query_tests {
    use std::{collections::HashSet, time::Duration};

    use chrono::{DateTime, Days, NaiveDate, Utc};

    use tokio::test;
    use uuid::Uuid;

    use super::{TaskQueryOptions, query_tasks_inner};
    use crate::db::test_utils::{create_test_tag, create_test_task, get_pool, get_time};
    use crate::{
        com::constants::MAX_LIMIT,
        db::filters::{DateBound, DateFilter, TaskSort},
        routes::models::{SortOrder, Start, tag::Model as TagCreate, task::Model as TaskCreate},
    };

    #[test]
    async fn default_query() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = get_time();

        // create test data
        let _test_tasks = [
            create_test_task(
                &mut tx,
                TaskCreate::default(),
                None,
                None,
                base_time,
                base_time,
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate::default(),
                None,
                None,
                base_time,
                base_time,
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate::default(),
                None,
                None,
                base_time,
                base_time + Duration::from_hours(1),
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate::default(),
                None,
                None,
                base_time,
                base_time + Duration::from_hours(1),
            )
            .await,
        ];

        let opts = TaskQueryOptions::default();

        let res = query_tasks_inner(&mut tx, Uuid::nil(), opts).await;
        assert!(res.is_ok(), "query should always succeed");
        if let Ok(tasks) = res {
            assert!(!tasks.is_empty(), "must have data to test on");

            assert!(
                tasks.is_sorted_by(|a, b| {
                    if a.updated_at > b.updated_at {
                        // updated_at is descending
                        true
                    } else if a.updated_at < b.updated_at {
                        // updated_at not descending
                        false
                    } else {
                        // updated_at equal; check id is ascending
                        assert_ne!(a.id, b.id, "no duplicate Tasks should have been returned");

                        a.id < b.id
                    }
                }),
                "default query should have returned with updated_at descending, with id ascending as fallback"
            );
            assert!(
                !tasks.iter().any(|task| task.deleted_at.is_some()),
                "query should not return any deleted Tasks"
            );
        }
    }

    #[test]
    async fn sort_updated_ascending() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = get_time();

        // create test data
        let _test_tasks = [
            create_test_task(
                &mut tx,
                TaskCreate::default(),
                None,
                None,
                base_time,
                base_time,
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate::default(),
                None,
                None,
                base_time,
                base_time,
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate::default(),
                None,
                None,
                base_time,
                base_time + Duration::from_hours(1),
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate::default(),
                None,
                None,
                base_time,
                base_time + Duration::from_hours(1),
            )
            .await,
        ];

        let opts = TaskQueryOptions {
            sort_order: TaskSort::Updated(SortOrder::Ascending),
            ..Default::default()
        };

        let res = query_tasks_inner(&mut tx, Uuid::nil(), opts).await;
        assert!(res.is_ok(), "query should always succeed");
        if let Ok(tasks) = res {
            assert!(!tasks.is_empty(), "must have data to test on");

            assert!(
                tasks.is_sorted_by(|a, b| {
                    if a.updated_at < b.updated_at {
                        // updated_at ascending
                        true
                    } else if a.updated_at > b.updated_at {
                        // updated_at not ascending
                        false
                    } else {
                        // updated_at is equal; check id is ascending
                        assert_ne!(a.id, b.id, "no duplicate Tasks should have been returned");

                        a.id < b.id
                    }
                }),
                "expected sort by updated_at ascending, with id ascending as fallback"
            );
            assert!(
                !tasks.iter().any(|task| task.deleted_at.is_some()),
                "query should not return any deleted Tasks"
            );
        }
    }

    #[test]
    async fn sort_created_descending() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = get_time();

        // create test data
        let _test_tasks = [
            create_test_task(
                &mut tx,
                TaskCreate::default(),
                None,
                None,
                base_time,
                base_time,
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate::default(),
                None,
                None,
                base_time,
                base_time,
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate::default(),
                None,
                None,
                base_time + Duration::from_hours(1),
                base_time + Duration::from_hours(2),
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate::default(),
                None,
                None,
                base_time + Duration::from_hours(1),
                base_time + Duration::from_hours(2),
            )
            .await,
        ];

        let opts = TaskQueryOptions {
            sort_order: TaskSort::Created(SortOrder::Descending),
            ..Default::default()
        };

        let res = query_tasks_inner(&mut tx, Uuid::nil(), opts).await;
        assert!(res.is_ok(), "query should always succeed");
        if let Ok(tasks) = res {
            assert!(!tasks.is_empty(), "must have data to test on");

            assert!(
                tasks.is_sorted_by(|a, b| {
                    if a.created_at > b.created_at {
                        // created_at descending
                        true
                    } else if a.created_at < b.created_at {
                        // created_at not descending
                        false
                    } else {
                        // created_at is equal; check id is ascending
                        assert_ne!(a.id, b.id, "no duplicate Tasks should have been returned");

                        a.id < b.id
                    }
                }),
                "expected sort by created_at descending, with id ascending as fallback"
            );
            assert!(
                !tasks.iter().any(|task| task.deleted_at.is_some()),
                "query should not return any deleted Tasks"
            );
        }
    }

    #[test]
    async fn sort_created_ascending() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = get_time();

        // create test data
        let _test_tasks = [
            create_test_task(
                &mut tx,
                TaskCreate::default(),
                None,
                None,
                base_time,
                base_time,
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate::default(),
                None,
                None,
                base_time,
                base_time,
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate::default(),
                None,
                None,
                base_time + Duration::from_hours(1),
                base_time + Duration::from_hours(1),
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate::default(),
                None,
                None,
                base_time + Duration::from_hours(2),
                base_time + Duration::from_hours(2),
            )
            .await,
        ];

        let opts = TaskQueryOptions {
            sort_order: TaskSort::Created(SortOrder::Ascending),
            ..Default::default()
        };

        let res = query_tasks_inner(&mut tx, Uuid::nil(), opts).await;
        assert!(res.is_ok(), "query should always succeed");
        if let Ok(tasks) = res {
            assert!(!tasks.is_empty(), "must have data to test on");

            assert!(
                tasks.is_sorted_by(|a, b| {
                    if a.created_at < b.created_at {
                        // created_at ascending
                        true
                    } else if a.created_at > b.created_at {
                        // created_at not ascending
                        false
                    } else {
                        // created_at is equal; check id is ascending
                        assert_ne!(a.id, b.id, "no duplicate Tasks should have been returned");

                        a.id < b.id
                    }
                }),
                "expected sort by created_at ascending, with id ascending as fallback"
            );
            assert!(
                !tasks.iter().any(|task| task.deleted_at.is_some()),
                "query should not return any deleted Tasks"
            );
        }
    }

    #[test]
    async fn sort_title_ascending() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = get_time();

        // create test data
        let _test_tasks = [
            create_test_task(
                &mut tx,
                TaskCreate {
                    title: String::new(),
                    ..Default::default()
                },
                None,
                None,
                base_time,
                base_time,
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate {
                    title: String::new(),
                    ..Default::default()
                },
                None,
                None,
                base_time,
                base_time,
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate {
                    title: String::new(),
                    ..Default::default()
                },
                None,
                None,
                base_time,
                base_time + Duration::from_hours(1),
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate {
                    title: String::new(),
                    ..Default::default()
                },
                None,
                None,
                base_time,
                base_time + Duration::from_hours(2),
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate {
                    title: "Task A".to_string(),
                    ..Default::default()
                },
                None,
                None,
                base_time,
                base_time,
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate {
                    title: "Task B".to_string(),
                    ..Default::default()
                },
                None,
                None,
                base_time,
                base_time,
            )
            .await,
        ];

        let opts = TaskQueryOptions {
            sort_order: TaskSort::Title(SortOrder::Ascending),
            ..Default::default()
        };

        let res = query_tasks_inner(&mut tx, Uuid::nil(), opts).await;
        assert!(res.is_ok(), "query should always succeed");
        if let Ok(tasks) = res {
            assert!(!tasks.is_empty(), "must have data to test on");

            assert!(
                tasks.is_sorted_by(|a, b| {
                    if a.title < b.title {
                        // title ascending
                        true
                    } else if a.title > b.title {
                        // title not ascending
                        false
                    } else if a.updated_at > b.updated_at {
                        // created_at is equal; update_at is descending
                        true
                    } else if a.updated_at < b.updated_at {
                        // created_at is equal; update_at is not descending
                        false
                    } else {
                        assert_ne!(a.id, b.id, "no duplicate Tasks should have been returned");

                        a.id < b.id
                    }
                }),
                "expected sort by title ascending, with updated_at descending then id ascending as fallbacks"
            );
            assert!(
                !tasks.iter().any(|task| task.deleted_at.is_some()),
                "query should not return any deleted Tasks"
            );
        }
    }

    #[test]
    async fn sort_title_descending() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = get_time();

        // create test data
        let _test_tasks = [
            create_test_task(
                &mut tx,
                TaskCreate {
                    title: String::new(),
                    ..Default::default()
                },
                None,
                None,
                base_time,
                base_time,
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate {
                    title: String::new(),
                    ..Default::default()
                },
                None,
                None,
                base_time,
                base_time,
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate {
                    title: String::new(),
                    ..Default::default()
                },
                None,
                None,
                base_time,
                base_time + Duration::from_hours(1),
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate {
                    title: String::new(),
                    ..Default::default()
                },
                None,
                None,
                base_time,
                base_time + Duration::from_hours(2),
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate {
                    title: "Task A".to_string(),
                    ..Default::default()
                },
                None,
                None,
                base_time,
                base_time,
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate {
                    title: "Task B".to_string(),
                    ..Default::default()
                },
                None,
                None,
                base_time,
                base_time,
            )
            .await,
        ];

        let opts = TaskQueryOptions {
            sort_order: TaskSort::Title(SortOrder::Descending),
            ..Default::default()
        };

        let res = query_tasks_inner(&mut tx, Uuid::nil(), opts).await;
        assert!(res.is_ok(), "query should always succeed");
        if let Ok(tasks) = res {
            assert!(!tasks.is_empty(), "must have data to test on");

            assert!(
                tasks.is_sorted_by(|a, b| {
                    if a.title > b.title {
                        // title descending
                        true
                    } else if a.title < b.title {
                        // title not descending
                        false
                    } else if a.updated_at > b.updated_at {
                        // created_at is equal; update_at is descending
                        true
                    } else if a.updated_at < b.updated_at {
                        // created_at is equal; update_at is not descending
                        false
                    } else {
                        assert_ne!(a.id, b.id, "no duplicate Tasks should have been returned");

                        a.id < b.id
                    }
                }),
                "expected sort by title descending, with updated_at descending then id ascending as fallbacks"
            );
            assert!(
                !tasks.iter().any(|task| task.deleted_at.is_some()),
                "query should not return any deleted Tasks"
            );
        }
    }

    #[test]
    async fn sort_start_ascending() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = get_time();

        // create test data
        let _test_tasks = [
            create_test_task(
                &mut tx,
                TaskCreate::default(),
                None,
                None,
                base_time,
                base_time,
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate::default(),
                None,
                None,
                base_time,
                base_time,
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate {
                    start: Some(Start::On(NaiveDate::from_ymd_opt(2026, 1, 1).unwrap())),
                    ..Default::default()
                },
                None,
                None,
                base_time,
                base_time,
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate {
                    start: Some(Start::On(NaiveDate::from_ymd_opt(2026, 1, 2).unwrap())),
                    ..Default::default()
                },
                None,
                None,
                base_time,
                base_time,
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate {
                    start: Some(Start::At(
                        NaiveDate::from_ymd_opt(2026, 1, 1)
                            .unwrap()
                            .and_hms_opt(0, 0, 0)
                            .unwrap()
                            .and_utc(),
                    )),
                    ..Default::default()
                },
                None,
                None,
                base_time,
                base_time,
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate {
                    start: Some(Start::At(
                        NaiveDate::from_ymd_opt(2026, 1, 2)
                            .unwrap()
                            .and_hms_opt(0, 0, 0)
                            .unwrap()
                            .and_utc(),
                    )),
                    ..Default::default()
                },
                None,
                None,
                base_time,
                base_time,
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate::default(),
                None,
                None,
                base_time,
                base_time + Duration::from_hours(1),
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate::default(),
                None,
                None,
                base_time,
                base_time + Duration::from_hours(2),
            )
            .await,
        ];

        let opts = TaskQueryOptions {
            sort_order: TaskSort::Start(SortOrder::Ascending),
            ..Default::default()
        };

        let res = query_tasks_inner(&mut tx, Uuid::nil(), opts).await;
        assert!(res.is_ok(), "query should always succeed");
        if let Ok(tasks) = res {
            assert!(!tasks.is_empty(), "must have data to test on");

            let mut seen_nulls = false;
            assert!(
                tasks.is_sorted_by(|a, b| {
                    let start_a = a.start_dt.map(|dt| dt.date_naive());
                    let start_b = b.start_dt.map(|dt| dt.date_naive());

                    if seen_nulls && (start_a.is_some() || start_b.is_some()) {
                        panic!("found dates within NULL section of sorted tasks");
                    }

                    if start_a.is_some() && start_b.is_none() {
                        // at NULL transition
                        seen_nulls = true;

                        return true;
                    }

                    if start_a < start_b {
                        // start ascending
                        true
                    } else if start_a > start_b {
                        // start not ascending
                        false
                    } else if a.updated_at > b.updated_at {
                        // start is equal; update_at is descending
                        true
                    } else if a.updated_at < b.updated_at {
                        // start is equal; update_at is not descending
                        false
                    } else {
                        assert_ne!(a.id, b.id, "no duplicate Tasks should have been returned");

                        a.id < b.id
                    }
                }),
                "expected sort by start ascending, with updated_at descending then id ascending as fallbacks: {:?}",
                tasks
                    .iter()
                    .map(|task| task.start_dt)
                    .collect::<Vec<Option<DateTime<Utc>>>>()
            );
            assert!(
                !tasks.iter().any(|task| task.deleted_at.is_some()),
                "query should not return any deleted Tasks"
            );
        }
    }

    #[test]
    async fn sort_start_descending() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = get_time();

        // create test data
        let _test_tasks = [
            create_test_task(
                &mut tx,
                TaskCreate::default(),
                None,
                None,
                base_time,
                base_time,
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate::default(),
                None,
                None,
                base_time,
                base_time,
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate {
                    start: Some(Start::On(NaiveDate::from_ymd_opt(2026, 1, 1).unwrap())),
                    ..Default::default()
                },
                None,
                None,
                base_time,
                base_time,
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate {
                    start: Some(Start::On(NaiveDate::from_ymd_opt(2026, 1, 2).unwrap())),
                    ..Default::default()
                },
                None,
                None,
                base_time,
                base_time,
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate {
                    start: Some(Start::At(
                        NaiveDate::from_ymd_opt(2026, 1, 1)
                            .unwrap()
                            .and_hms_opt(0, 0, 0)
                            .unwrap()
                            .and_utc(),
                    )),
                    ..Default::default()
                },
                None,
                None,
                base_time,
                base_time,
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate {
                    start: Some(Start::At(
                        NaiveDate::from_ymd_opt(2026, 1, 2)
                            .unwrap()
                            .and_hms_opt(0, 0, 0)
                            .unwrap()
                            .and_utc(),
                    )),
                    ..Default::default()
                },
                None,
                None,
                base_time,
                base_time,
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate::default(),
                None,
                None,
                base_time,
                base_time + Duration::from_hours(1),
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate::default(),
                None,
                None,
                base_time,
                base_time + Duration::from_hours(2),
            )
            .await,
        ];

        let opts = TaskQueryOptions {
            sort_order: TaskSort::Start(SortOrder::Descending),
            ..Default::default()
        };

        let res = query_tasks_inner(&mut tx, Uuid::nil(), opts).await;
        assert!(res.is_ok(), "query should always succeed");
        if let Ok(tasks) = res {
            assert!(!tasks.is_empty(), "must have data to test on");

            let mut seen_nulls = false;
            assert!(
                tasks.is_sorted_by(|a, b| {
                    let start_a = a.start_dt.map(|dt| dt.date_naive());
                    let start_b = b.start_dt.map(|dt| dt.date_naive());

                    if seen_nulls && (start_a.is_some() || start_b.is_some()) {
                        panic!("found dates within NULL section of sorted tasks");
                    }

                    if start_a.is_some() && start_b.is_none() {
                        // at NULL transition
                        seen_nulls = true;

                        return true;
                    }

                    if start_a > start_b {
                        // start descending
                        true
                    } else if start_a < start_b {
                        // start not descending
                        false
                    } else if a.updated_at > b.updated_at {
                        // start is equal; update_at is descending
                        true
                    } else if a.updated_at < b.updated_at {
                        // start is equal; update_at is not descending
                        false
                    } else {
                        assert_ne!(a.id, b.id, "no duplicate Tasks should have been returned");

                        a.id < b.id
                    }
                }),
                "expected sort by start descending, with updated_at descending then id ascending as fallbacks"
            );
            assert!(
                !tasks.iter().any(|task| task.deleted_at.is_some()),
                "query should not return any deleted Tasks"
            );
        }
    }

    #[test]
    async fn sort_deadline_ascending() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = get_time();

        // create test data
        let _test_tasks = [
            create_test_task(
                &mut tx,
                TaskCreate::default(),
                None,
                None,
                base_time,
                base_time,
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate::default(),
                None,
                None,
                base_time,
                base_time,
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate {
                    deadline: Some(NaiveDate::from_ymd_opt(2026, 1, 1).unwrap()),
                    ..Default::default()
                },
                None,
                None,
                base_time,
                base_time,
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate {
                    deadline: Some(NaiveDate::from_ymd_opt(2026, 1, 2).unwrap()),
                    ..Default::default()
                },
                None,
                None,
                base_time,
                base_time,
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate::default(),
                None,
                None,
                base_time,
                base_time + Duration::from_hours(1),
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate::default(),
                None,
                None,
                base_time,
                base_time + Duration::from_hours(2),
            )
            .await,
        ];

        let opts = TaskQueryOptions {
            sort_order: TaskSort::Deadline(SortOrder::Ascending),
            ..Default::default()
        };

        let res = query_tasks_inner(&mut tx, Uuid::nil(), opts).await;
        assert!(res.is_ok(), "query should always succeed");
        if let Ok(tasks) = res {
            assert!(!tasks.is_empty(), "must have data to test on");

            let mut seen_nulls = false;
            assert!(
                tasks.is_sorted_by(|a, b| {
                    if seen_nulls && (a.deadline.is_some() || b.deadline.is_some()) {
                        panic!("found dates within NULL section of sorted tasks");
                    }

                    if a.deadline.is_some() && b.deadline.is_none() {
                        // at NULL transition
                        seen_nulls = true;

                        return true;
                    }

                    if a.deadline < b.deadline {
                        // daedline ascending
                        true
                    } else if a.deadline > b.deadline {
                        // deadline not ascending
                        false
                    } else if a.updated_at > b.updated_at {
                        // deadline is equal; update_at is descending
                        true
                    } else if a.updated_at < b.updated_at {
                        // deadline is equal; update_at is not descending
                        false
                    } else {
                        assert_ne!(a.id, b.id, "no duplicate Tasks should have been returned");

                        a.id < b.id
                    }
                }),
                "expected sort by deadline ascending, with updated_at descending then id ascending as fallbacks",
            );
            assert!(
                !tasks.iter().any(|task| task.deleted_at.is_some()),
                "query should not return any deleted Tasks"
            );
        }
    }

    #[test]
    async fn sort_deadline_descending() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = get_time();

        // create test data
        let _test_tasks = [
            create_test_task(
                &mut tx,
                TaskCreate::default(),
                None,
                None,
                base_time,
                base_time,
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate::default(),
                None,
                None,
                base_time,
                base_time,
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate {
                    deadline: Some(NaiveDate::from_ymd_opt(2026, 1, 1).unwrap()),
                    ..Default::default()
                },
                None,
                None,
                base_time,
                base_time,
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate {
                    deadline: Some(NaiveDate::from_ymd_opt(2026, 1, 2).unwrap()),
                    ..Default::default()
                },
                None,
                None,
                base_time,
                base_time,
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate::default(),
                None,
                None,
                base_time,
                base_time + Duration::from_hours(1),
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate::default(),
                None,
                None,
                base_time,
                base_time + Duration::from_hours(2),
            )
            .await,
        ];

        let opts = TaskQueryOptions {
            sort_order: TaskSort::Deadline(SortOrder::Descending),
            ..Default::default()
        };

        let res = query_tasks_inner(&mut tx, Uuid::nil(), opts).await;
        assert!(res.is_ok(), "query should always succeed");
        if let Ok(tasks) = res {
            assert!(!tasks.is_empty(), "must have data to test on");

            let mut seen_nulls = false;
            assert!(
                tasks.is_sorted_by(|a, b| {
                    if seen_nulls && (a.deadline.is_some() || b.deadline.is_some()) {
                        panic!("found dates within NULL section of sorted tasks");
                    }

                    if a.deadline.is_some() && b.deadline.is_none() {
                        // at NULL transition
                        seen_nulls = true;

                        return true;
                    }

                    if a.deadline > b.deadline {
                        // daedline descending
                        true
                    } else if a.deadline < b.deadline {
                        // deadline not descending
                        false
                    } else if a.updated_at > b.updated_at {
                        // deadline is equal; update_at is descending
                        true
                    } else if a.updated_at < b.updated_at {
                        // deadline is equal; update_at is not descending
                        false
                    } else {
                        assert_ne!(a.id, b.id, "no duplicate Tasks should have been returned");

                        a.id < b.id
                    }
                }),
                "expected sort by deadline descending, with updated_at descending then id ascending as fallbacks",
            );
            assert!(
                !tasks.iter().any(|task| task.deleted_at.is_some()),
                "query should not return any deleted Tasks"
            );
        }
    }

    #[test]
    async fn limit() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = get_time();

        // create test data
        for _ in 0..50 {
            create_test_task(
                &mut tx,
                TaskCreate::default(),
                None,
                None,
                base_time,
                base_time,
            )
            .await;
        }

        for i in 1..=50 {
            let opts = TaskQueryOptions {
                limit: Some(i),
                ..Default::default()
            };

            let res = query_tasks_inner(&mut tx, Uuid::nil(), opts).await;
            assert!(res.is_ok(), "query should always succeed");
            if let Ok(tasks) = res {
                assert!(!tasks.is_empty(), "must have data to test on");

                assert_eq!(
                    tasks.len(),
                    i as usize,
                    "limit does not match query filter value: {}",
                    i
                );
            }
        }
    }

    #[test]
    async fn limit_0() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = get_time();

        // create test data
        for _ in 0..50 {
            create_test_task(
                &mut tx,
                TaskCreate::default(),
                None,
                None,
                base_time,
                base_time,
            )
            .await;
        }

        let opts = TaskQueryOptions {
            limit: Some(0),
            ..Default::default()
        };

        let res = query_tasks_inner(&mut tx, Uuid::nil(), opts).await;
        assert!(res.is_ok(), "query should always succeed");
        if let Ok(tasks) = res {
            assert!(!tasks.is_empty(), "must have data to test on");

            assert_eq!(tasks.len(), 1, "minimum limit should be clamped to 1");
        }
    }

    #[test]
    async fn limit_absurdly_large() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = get_time();

        // create test data
        for _ in 0..250 {
            create_test_task(
                &mut tx,
                TaskCreate::default(),
                None,
                None,
                base_time,
                base_time,
            )
            .await;
        }

        let opts = TaskQueryOptions {
            limit: Some(i64::MAX),
            ..Default::default()
        };

        let res = query_tasks_inner(&mut tx, Uuid::nil(), opts).await;
        assert!(res.is_ok(), "query should always succeed");
        if let Ok(tasks) = res {
            assert!(!tasks.is_empty(), "must have data to test on");

            assert_eq!(
                tasks.len(),
                MAX_LIMIT as usize,
                "maximum limit should be clamped to MAX_LIMIT ({})",
                MAX_LIMIT
            );
        }
    }

    #[test]
    async fn limit_negative() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = get_time();

        // create test data
        for _ in 0..10 {
            create_test_task(
                &mut tx,
                TaskCreate::default(),
                None,
                None,
                base_time,
                base_time,
            )
            .await;
        }

        let opts = TaskQueryOptions {
            limit: Some(-1),
            ..Default::default()
        };

        let res = query_tasks_inner(&mut tx, Uuid::nil(), opts).await;
        assert!(res.is_ok(), "query should always succeed");
        if let Ok(tasks) = res {
            assert!(!tasks.is_empty(), "must have data to test on");

            assert_eq!(tasks.len(), 1, "minimum limit should be clamped to 1");
        }

        let opts = TaskQueryOptions {
            limit: Some(-50),
            ..Default::default()
        };

        let res = query_tasks_inner(&mut tx, Uuid::nil(), opts).await;
        assert!(res.is_ok(), "query should always succeed");
        if let Ok(tasks) = res {
            assert!(!tasks.is_empty(), "must have data to test on");

            assert_eq!(tasks.len(), 1, "minimum limit should be clamped to 1");
        }

        let opts = TaskQueryOptions {
            limit: Some(i64::MIN),
            ..Default::default()
        };

        let res = query_tasks_inner(&mut tx, Uuid::nil(), opts).await;
        assert!(res.is_ok(), "query should always succeed");
        if let Ok(tasks) = res {
            assert!(!tasks.is_empty(), "must have data to test on");

            assert_eq!(tasks.len(), 1, "minimum limit should be clamped to 1");
        }
    }

    #[test]
    async fn limit_with_lots_of_data() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = get_time();

        // create test data
        for _ in 0..1000 {
            create_test_task(
                &mut tx,
                TaskCreate::default(),
                None,
                None,
                base_time,
                base_time,
            )
            .await;
        }

        let opts = TaskQueryOptions {
            limit: Some(i64::MAX),
            ..Default::default()
        };

        let res = query_tasks_inner(&mut tx, Uuid::nil(), opts).await;
        assert!(res.is_ok(), "query should always succeed");
        if let Ok(tasks) = res {
            assert!(!tasks.is_empty(), "must have data to test on");

            assert_eq!(
                tasks.len(),
                MAX_LIMIT as usize,
                "maximum limit should be clamped to MAX_LIMIT ({})",
                MAX_LIMIT
            );
        }
    }

    #[test]
    async fn limit_with_paging_offset() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = get_time();

        // create test data
        for _ in 0..254 {
            create_test_task(
                &mut tx,
                TaskCreate::default(),
                None,
                None,
                base_time,
                base_time,
            )
            .await;
        }

        let limit = 5;

        // keep paging until less than 'limit' tasks are return
        let mut i = 0;
        let mut seen = HashSet::new();
        loop {
            let opts = TaskQueryOptions {
                limit: Some(limit),
                offset: Some(i * limit),
                ..Default::default()
            };

            let res = query_tasks_inner(&mut tx, Uuid::nil(), opts).await;
            assert!(res.is_ok(), "query should always succeed");
            if let Ok(tasks) = res {
                if i == 0 {
                    assert!(
                        !tasks.is_empty(),
                        "first iteration: must have data to test on"
                    );
                }

                assert!(tasks.len() <= limit as usize);
                for task in &tasks {
                    assert!(seen.insert(task.id), "duplicate task encountered");
                }
                seen.extend(tasks.iter().map(|t| t.id));

                i += 1;

                if tasks.len() < limit as usize {
                    break;
                }
            }
        }

        // perform one more query to ensure that the end has been reached
        let opts = TaskQueryOptions {
            limit: Some(limit),
            offset: Some(i * limit),
            ..Default::default()
        };

        let res = query_tasks_inner(&mut tx, Uuid::nil(), opts).await;
        assert!(res.is_ok(), "query should always succeed");
        if let Ok(tasks) = res {
            assert!(tasks.is_empty(), "should be past the end of the list");
        }
    }

    #[test]
    async fn offset_absurdly_large() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = get_time();

        // create test data
        for _ in 0..250 {
            create_test_task(
                &mut tx,
                TaskCreate::default(),
                None,
                None,
                base_time,
                base_time,
            )
            .await;
        }

        let opts = TaskQueryOptions {
            offset: Some(i64::MAX),
            ..Default::default()
        };

        let res = query_tasks_inner(&mut tx, Uuid::nil(), opts).await;
        assert!(res.is_ok(), "query should always succeed");
    }

    #[test]
    async fn offset_negative() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = get_time();

        // create test data
        for _ in 0..250 {
            create_test_task(
                &mut tx,
                TaskCreate::default(),
                None,
                None,
                base_time,
                base_time,
            )
            .await;
        }

        let opts = TaskQueryOptions {
            offset: Some(-1),
            ..Default::default()
        };

        let res = query_tasks_inner(&mut tx, Uuid::nil(), opts).await;
        assert!(res.is_ok(), "query should always succeed");
        let tasks_1 = res.unwrap();
        assert!(!tasks_1.is_empty(), "must have data to test on");

        let opts = TaskQueryOptions {
            offset: Some(-50),
            ..Default::default()
        };

        let res = query_tasks_inner(&mut tx, Uuid::nil(), opts).await;
        assert!(res.is_ok(), "query should always succeed");
        let tasks_2 = res.unwrap();
        assert_eq!(
            tasks_1, tasks_2,
            "should match as negative offsets default to offset of 0"
        );

        let opts = TaskQueryOptions {
            offset: Some(i64::MIN),
            ..Default::default()
        };

        let res = query_tasks_inner(&mut tx, Uuid::nil(), opts).await;
        assert!(res.is_ok(), "query should always succeed");
        let tasks_3 = res.unwrap();
        assert_eq!(
            tasks_2, tasks_3,
            "should match as negative offsets default to offset of 0"
        )
    }

    #[test]
    async fn offset_without_limits() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = get_time();

        // create test data
        for _ in 0..250 {
            create_test_task(
                &mut tx,
                TaskCreate::default(),
                None,
                None,
                base_time,
                base_time,
            )
            .await;
        }

        let opts = TaskQueryOptions {
            offset: Some(20),
            ..Default::default()
        };

        let res = query_tasks_inner(&mut tx, Uuid::nil(), opts).await;
        assert!(res.is_ok(), "query should always succeed");
        if let Ok(tasks) = res {
            assert!(!tasks.is_empty(), "must have data to test on");

            let opts = TaskQueryOptions {
                limit: Some(70),
                ..Default::default()
            };

            let ref_tasks = query_tasks_inner(&mut tx, Uuid::nil(), opts).await.unwrap();

            assert_eq!(
                tasks,
                ref_tasks[20..],
                "should return 50 tasks offset by 20 tasks"
            )
        }
    }

    #[test]
    async fn filter_completed() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = get_time();

        // create test data
        let _test_tasks = [
            create_test_task(
                &mut tx,
                TaskCreate::default(),
                None,
                None,
                base_time,
                base_time,
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate::default(),
                Some(base_time),
                None,
                base_time,
                base_time,
            )
            .await,
        ];

        let res = query_tasks_inner(&mut tx, Uuid::nil(), TaskQueryOptions::default()).await;
        assert!(res.is_ok(), "query should always succeed");
        if let Ok(tasks) = res {
            assert!(!tasks.is_empty(), "must have data to test on");

            assert!(
                !tasks.iter().any(|task| task.completed_at.is_some()),
                "default should be `false` and only return uncompleted tasks"
            );
        }

        let opts = TaskQueryOptions {
            completed: true,
            ..Default::default()
        };

        let res = query_tasks_inner(&mut tx, Uuid::nil(), opts).await;
        assert!(res.is_ok(), "query should always succeed");
        if let Ok(tasks) = res {
            assert!(!tasks.is_empty(), "must have data to test on");

            assert!(
                tasks.iter().all(|task| task.completed_at.is_some()),
                "should only return completed tasks"
            );
        }
    }

    #[test]
    async fn filter_deleted() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = get_time();

        // create test data
        let _test_tasks = [
            create_test_task(
                &mut tx,
                TaskCreate::default(),
                None,
                None,
                base_time,
                base_time,
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate::default(),
                None,
                Some(base_time),
                base_time,
                base_time,
            )
            .await,
        ];

        let res = query_tasks_inner(&mut tx, Uuid::nil(), TaskQueryOptions::default()).await;
        assert!(res.is_ok(), "query should always succeed");
        if let Ok(tasks) = res {
            assert!(!tasks.is_empty(), "must have data to test on");

            assert!(
                !tasks.iter().any(|task| task.deleted_at.is_some()),
                "default should be `false` and only return not deleted tasks"
            )
        }

        let opts = TaskQueryOptions {
            deleted: true,
            ..Default::default()
        };

        let res = query_tasks_inner(&mut tx, Uuid::nil(), opts).await;
        assert!(res.is_ok(), "query should always succeed");
        if let Ok(tasks) = res {
            assert!(!tasks.is_empty(), "must have data to test on");

            assert!(
                tasks.iter().all(|task| task.deleted_at.is_some()),
                "should only return deleted tasks"
            )
        }
    }

    #[test]
    async fn filter_has_start() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = get_time();

        // create test data
        let _test_tasks = [
            create_test_task(
                &mut tx,
                TaskCreate::default(),
                None,
                None,
                base_time,
                base_time,
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate {
                    start: Some(Start::On(base_time.date_naive())),
                    ..Default::default()
                },
                None,
                None,
                base_time,
                base_time,
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate {
                    start: Some(Start::At(base_time)),
                    ..Default::default()
                },
                None,
                None,
                base_time,
                base_time,
            )
            .await,
        ];

        let opts = TaskQueryOptions {
            start_filter: Some(DateFilter::Exists(true)),
            ..Default::default()
        };

        let res = query_tasks_inner(&mut tx, Uuid::nil(), opts).await;
        assert!(res.is_ok(), "query should always succeed");
        if let Ok(tasks) = res {
            assert!(!tasks.is_empty(), "must have data to test on");

            assert!(
                tasks.iter().all(|task| task.start_dt.is_some()),
                "should only return tasks with start date"
            );
        }

        let opts = TaskQueryOptions {
            start_filter: Some(DateFilter::Exists(false)),
            ..Default::default()
        };

        let res = query_tasks_inner(&mut tx, Uuid::nil(), opts).await;
        assert!(res.is_ok());
        if let Ok(tasks) = res {
            assert!(!tasks.is_empty(), "must have data to test on");

            assert!(
                tasks.iter().all(|task| task.start_dt.is_none()),
                "should only return tasks without start date"
            );
        }
    }

    #[test]
    async fn filter_start_on() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = get_time();

        // create test data
        let _test_tasks = [
            create_test_task(
                &mut tx,
                TaskCreate::default(),
                None,
                None,
                base_time,
                base_time,
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate {
                    start: Some(Start::On((base_time - Days::new(1)).date_naive())),
                    ..Default::default()
                },
                None,
                None,
                base_time,
                base_time,
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate {
                    start: Some(Start::At(base_time - Days::new(1))),
                    ..Default::default()
                },
                None,
                None,
                base_time,
                base_time,
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate {
                    start: Some(Start::On(base_time.date_naive())),
                    ..Default::default()
                },
                None,
                None,
                base_time,
                base_time,
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate {
                    start: Some(Start::At(base_time)),
                    ..Default::default()
                },
                None,
                None,
                base_time,
                base_time,
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate {
                    start: Some(Start::On((base_time + Days::new(1)).date_naive())),
                    ..Default::default()
                },
                None,
                None,
                base_time,
                base_time,
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate {
                    start: Some(Start::At(base_time + Days::new(1))),
                    ..Default::default()
                },
                None,
                None,
                base_time,
                base_time,
            )
            .await,
        ];

        let opts = TaskQueryOptions {
            start_filter: Some(DateFilter::On(base_time.date_naive())),
            ..Default::default()
        };

        let res = query_tasks_inner(&mut tx, Uuid::nil(), opts).await;
        assert!(res.is_ok());
        if let Ok(tasks) = res {
            assert!(!tasks.is_empty(), "must have data to test on");

            assert!(
                tasks.iter().all(|task| if let Some(dt) = task.start_dt {
                    dt.date_naive() == base_time.date_naive()
                } else {
                    false
                }),
                "should only return tasks that have start date on given date"
            );
        }

        let opts = TaskQueryOptions {
            start_filter: Some(DateFilter::NotOn(base_time.date_naive())),
            ..Default::default()
        };

        let res = query_tasks_inner(&mut tx, Uuid::nil(), opts).await;
        assert!(res.is_ok());
        if let Ok(tasks) = res {
            assert!(!tasks.is_empty(), "must have data to test on");

            assert!(
                tasks.iter().all(|task| if let Some(dt) = task.start_dt {
                    dt.date_naive() != base_time.date_naive()
                } else {
                    true
                }),
                "should only return tasks that do not have start date on given date"
            );
        }
    }

    #[test]
    async fn filter_start_after_excl() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = get_time();

        // create test data
        let _test_tasks = [
            create_test_task(
                &mut tx,
                TaskCreate::default(),
                None,
                None,
                base_time,
                base_time,
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate {
                    start: Some(Start::On((base_time - Days::new(1)).date_naive())),
                    ..Default::default()
                },
                None,
                None,
                base_time,
                base_time,
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate {
                    start: Some(Start::At(base_time - Days::new(1))),
                    ..Default::default()
                },
                None,
                None,
                base_time,
                base_time,
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate {
                    start: Some(Start::On(base_time.date_naive())),
                    ..Default::default()
                },
                None,
                None,
                base_time,
                base_time,
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate {
                    start: Some(Start::At(base_time)),
                    ..Default::default()
                },
                None,
                None,
                base_time,
                base_time,
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate {
                    start: Some(Start::On((base_time + Days::new(1)).date_naive())),
                    ..Default::default()
                },
                None,
                None,
                base_time,
                base_time,
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate {
                    start: Some(Start::At(base_time + Days::new(1))),
                    ..Default::default()
                },
                None,
                None,
                base_time,
                base_time,
            )
            .await,
        ];

        let opts = TaskQueryOptions {
            start_filter: Some(DateFilter::StartRange(DateBound::Exclusive(
                base_time.date_naive(),
            ))),
            ..Default::default()
        };

        let res = query_tasks_inner(&mut tx, Uuid::nil(), opts).await;
        assert!(res.is_ok());
        if let Ok(tasks) = res {
            assert!(!tasks.is_empty(), "must have data to test on");

            assert!(
                tasks.iter().all(|task| if let Some(dt) = task.start_dt {
                    dt.date_naive() > base_time.date_naive()
                } else {
                    false
                }),
                "should only return tasks that have start after a given date excluding the given date"
            );
        }
    }

    #[test]
    async fn filter_start_after_incl() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = get_time();

        // create test data
        let _test_tasks = [
            create_test_task(
                &mut tx,
                TaskCreate::default(),
                None,
                None,
                base_time,
                base_time,
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate {
                    start: Some(Start::On((base_time - Days::new(1)).date_naive())),
                    ..Default::default()
                },
                None,
                None,
                base_time,
                base_time,
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate {
                    start: Some(Start::At(base_time - Days::new(1))),
                    ..Default::default()
                },
                None,
                None,
                base_time,
                base_time,
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate {
                    start: Some(Start::On(base_time.date_naive())),
                    ..Default::default()
                },
                None,
                None,
                base_time,
                base_time,
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate {
                    start: Some(Start::At(base_time)),
                    ..Default::default()
                },
                None,
                None,
                base_time,
                base_time,
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate {
                    start: Some(Start::On((base_time + Days::new(1)).date_naive())),
                    ..Default::default()
                },
                None,
                None,
                base_time,
                base_time,
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate {
                    start: Some(Start::At(base_time + Days::new(1))),
                    ..Default::default()
                },
                None,
                None,
                base_time,
                base_time,
            )
            .await,
        ];

        let opts = TaskQueryOptions {
            start_filter: Some(DateFilter::StartRange(DateBound::Inclusive(
                base_time.date_naive(),
            ))),
            ..Default::default()
        };

        let res = query_tasks_inner(&mut tx, Uuid::nil(), opts).await;
        assert!(res.is_ok());
        if let Ok(tasks) = res {
            assert!(!tasks.is_empty(), "must have data to test on");

            assert!(
                tasks.iter().all(|task| if let Some(dt) = task.start_dt {
                    dt.date_naive() >= base_time.date_naive()
                } else {
                    false
                }),
                "should only return tasks that have start after a given date including the given date"
            );
        }
    }

    #[test]
    async fn filter_start_before_excl() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = get_time();

        // create test data
        let _test_tasks = [
            create_test_task(
                &mut tx,
                TaskCreate::default(),
                None,
                None,
                base_time,
                base_time,
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate {
                    start: Some(Start::On((base_time - Days::new(1)).date_naive())),
                    ..Default::default()
                },
                None,
                None,
                base_time,
                base_time,
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate {
                    start: Some(Start::At(base_time - Days::new(1))),
                    ..Default::default()
                },
                None,
                None,
                base_time,
                base_time,
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate {
                    start: Some(Start::On(base_time.date_naive())),
                    ..Default::default()
                },
                None,
                None,
                base_time,
                base_time,
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate {
                    start: Some(Start::At(base_time)),
                    ..Default::default()
                },
                None,
                None,
                base_time,
                base_time,
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate {
                    start: Some(Start::On((base_time + Days::new(1)).date_naive())),
                    ..Default::default()
                },
                None,
                None,
                base_time,
                base_time,
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate {
                    start: Some(Start::At(base_time + Days::new(1))),
                    ..Default::default()
                },
                None,
                None,
                base_time,
                base_time,
            )
            .await,
        ];

        let opts = TaskQueryOptions {
            start_filter: Some(DateFilter::EndRange(DateBound::Exclusive(
                base_time.date_naive(),
            ))),
            ..Default::default()
        };

        let res = query_tasks_inner(&mut tx, Uuid::nil(), opts).await;
        assert!(res.is_ok());
        if let Ok(tasks) = res {
            assert!(!tasks.is_empty(), "must have data to test on");

            assert!(
                tasks.iter().all(|task| if let Some(dt) = task.start_dt {
                    dt.date_naive() < base_time.date_naive()
                } else {
                    false
                }),
                "should only return tasks that have start before a given date excluding the given date"
            );
        }
    }

    #[test]
    async fn filter_start_before_incl() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = get_time();

        // create test data
        let _test_tasks = [
            create_test_task(
                &mut tx,
                TaskCreate::default(),
                None,
                None,
                base_time,
                base_time,
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate {
                    start: Some(Start::On((base_time - Days::new(1)).date_naive())),
                    ..Default::default()
                },
                None,
                None,
                base_time,
                base_time,
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate {
                    start: Some(Start::At(base_time - Days::new(1))),
                    ..Default::default()
                },
                None,
                None,
                base_time,
                base_time,
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate {
                    start: Some(Start::On(base_time.date_naive())),
                    ..Default::default()
                },
                None,
                None,
                base_time,
                base_time,
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate {
                    start: Some(Start::At(base_time)),
                    ..Default::default()
                },
                None,
                None,
                base_time,
                base_time,
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate {
                    start: Some(Start::On((base_time + Days::new(1)).date_naive())),
                    ..Default::default()
                },
                None,
                None,
                base_time,
                base_time,
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate {
                    start: Some(Start::At(base_time + Days::new(1))),
                    ..Default::default()
                },
                None,
                None,
                base_time,
                base_time,
            )
            .await,
        ];

        let opts = TaskQueryOptions {
            start_filter: Some(DateFilter::EndRange(DateBound::Inclusive(
                base_time.date_naive(),
            ))),
            ..Default::default()
        };

        let res = query_tasks_inner(&mut tx, Uuid::nil(), opts).await;
        assert!(res.is_ok());
        if let Ok(tasks) = res {
            assert!(!tasks.is_empty(), "must have data to test on");

            assert!(
                tasks.iter().all(|task| if let Some(dt) = task.start_dt {
                    dt.date_naive() <= base_time.date_naive()
                } else {
                    false
                }),
                "should only return tasks that have start before a given date including the given date"
            );
        }
    }

    #[test]
    async fn filter_start_between() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = get_time();

        // create test data
        let _test_tasks = [
            create_test_task(
                &mut tx,
                TaskCreate::default(),
                None,
                None,
                base_time,
                base_time,
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate {
                    start: Some(Start::On((base_time - Days::new(2)).date_naive())),
                    ..Default::default()
                },
                None,
                None,
                base_time,
                base_time,
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate {
                    start: Some(Start::At(base_time - Days::new(2))),
                    ..Default::default()
                },
                None,
                None,
                base_time,
                base_time,
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate {
                    start: Some(Start::On((base_time - Days::new(1)).date_naive())),
                    ..Default::default()
                },
                None,
                None,
                base_time,
                base_time,
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate {
                    start: Some(Start::At(base_time - Days::new(1))),
                    ..Default::default()
                },
                None,
                None,
                base_time,
                base_time,
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate {
                    start: Some(Start::On(base_time.date_naive())),
                    ..Default::default()
                },
                None,
                None,
                base_time,
                base_time,
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate {
                    start: Some(Start::At(base_time)),
                    ..Default::default()
                },
                None,
                None,
                base_time,
                base_time,
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate {
                    start: Some(Start::On((base_time + Days::new(1)).date_naive())),
                    ..Default::default()
                },
                None,
                None,
                base_time,
                base_time,
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate {
                    start: Some(Start::At(base_time + Days::new(1))),
                    ..Default::default()
                },
                None,
                None,
                base_time,
                base_time,
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate {
                    start: Some(Start::On((base_time + Days::new(2)).date_naive())),
                    ..Default::default()
                },
                None,
                None,
                base_time,
                base_time,
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate {
                    start: Some(Start::At(base_time + Days::new(2))),
                    ..Default::default()
                },
                None,
                None,
                base_time,
                base_time,
            )
            .await,
        ];

        let opts = TaskQueryOptions {
            start_filter: Some(DateFilter::Range(
                DateBound::Exclusive((base_time - Days::new(1)).date_naive()),
                DateBound::Exclusive((base_time + Days::new(1)).date_naive()),
            )),
            ..Default::default()
        };

        let res = query_tasks_inner(&mut tx, Uuid::nil(), opts).await;
        assert!(res.is_ok());
        if let Ok(tasks) = res {
            assert!(!tasks.is_empty(), "must have data to test on");

            assert!(
                tasks.iter().all(|task| if let Some(dt) = task.start_dt {
                    dt.date_naive() > (base_time - Days::new(1)).date_naive()
                        && dt.date_naive() < (base_time + Days::new(1)).date_naive()
                } else {
                    false
                }),
                "should only return tasks that have start date between given range excluding both endpoints"
            )
        }

        let opts = TaskQueryOptions {
            start_filter: Some(DateFilter::Range(
                DateBound::Exclusive((base_time - Days::new(1)).date_naive()),
                DateBound::Inclusive((base_time + Days::new(1)).date_naive()),
            )),
            ..Default::default()
        };

        let res = query_tasks_inner(&mut tx, Uuid::nil(), opts).await;
        assert!(res.is_ok());
        if let Ok(tasks) = res {
            assert!(!tasks.is_empty(), "must have data to test on");

            assert!(
                tasks.iter().all(|task| if let Some(dt) = task.start_dt {
                    dt.date_naive() > (base_time - Days::new(1)).date_naive()
                        && dt.date_naive() <= (base_time + Days::new(1)).date_naive()
                } else {
                    false
                }),
                "should only return tasks that have start date between given range excluding only start endpoint"
            )
        }

        let opts = TaskQueryOptions {
            start_filter: Some(DateFilter::Range(
                DateBound::Inclusive((base_time - Days::new(1)).date_naive()),
                DateBound::Inclusive((base_time + Days::new(1)).date_naive()),
            )),
            ..Default::default()
        };

        let res = query_tasks_inner(&mut tx, Uuid::nil(), opts).await;
        assert!(res.is_ok());
        if let Ok(tasks) = res {
            assert!(!tasks.is_empty(), "must have data to test on");

            assert!(
                tasks.iter().all(|task| if let Some(dt) = task.start_dt {
                    dt.date_naive() >= (base_time - Days::new(1)).date_naive()
                        && dt.date_naive() <= (base_time + Days::new(1)).date_naive()
                } else {
                    false
                }),
                "should only return tasks that have start date between given range including both endpoints"
            )
        }

        let opts = TaskQueryOptions {
            start_filter: Some(DateFilter::Range(
                DateBound::Inclusive((base_time - Days::new(1)).date_naive()),
                DateBound::Exclusive((base_time + Days::new(1)).date_naive()),
            )),
            ..Default::default()
        };

        let res = query_tasks_inner(&mut tx, Uuid::nil(), opts).await;
        assert!(res.is_ok());
        if let Ok(tasks) = res {
            assert!(!tasks.is_empty(), "must have data to test on");

            assert!(
                tasks.iter().all(|task| if let Some(dt) = task.start_dt {
                    dt.date_naive() >= (base_time - Days::new(1)).date_naive()
                        && dt.date_naive() < (base_time + Days::new(1)).date_naive()
                } else {
                    false
                }),
                "should only return tasks that have start date between given range excluding only end endpoint"
            )
        }
    }

    #[test]
    async fn filter_has_deadline() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = get_time();

        // create test data
        let _test_tasks = [
            create_test_task(
                &mut tx,
                TaskCreate::default(),
                None,
                None,
                base_time,
                base_time,
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate {
                    deadline: Some((base_time - Days::new(1)).date_naive()),
                    ..Default::default()
                },
                None,
                None,
                base_time,
                base_time,
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate {
                    deadline: Some(base_time.date_naive()),
                    ..Default::default()
                },
                None,
                None,
                base_time,
                base_time,
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate {
                    deadline: Some((base_time + Days::new(1)).date_naive()),
                    ..Default::default()
                },
                None,
                None,
                base_time,
                base_time,
            )
            .await,
        ];

        let opts = TaskQueryOptions {
            deadline_filter: Some(DateFilter::Exists(true)),
            ..Default::default()
        };

        let res = query_tasks_inner(&mut tx, Uuid::nil(), opts).await;
        assert!(res.is_ok());
        if let Ok(tasks) = res {
            assert!(!tasks.is_empty(), "must have data to test on");

            assert!(
                tasks.iter().all(|task| task.deadline.is_some()),
                "should only return tasks with deadlines"
            )
        }

        let opts = TaskQueryOptions {
            deadline_filter: Some(DateFilter::Exists(false)),
            ..Default::default()
        };

        let res = query_tasks_inner(&mut tx, Uuid::nil(), opts).await;
        assert!(res.is_ok());
        if let Ok(tasks) = res {
            assert!(!tasks.is_empty(), "must have data to test on");

            assert!(
                tasks.iter().all(|task| task.deadline.is_none()),
                "should only return tasks without deadlines"
            )
        }
    }

    #[test]
    async fn filter_deadline_on() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = get_time();

        // create test data
        let _test_tasks = [
            create_test_task(
                &mut tx,
                TaskCreate::default(),
                None,
                None,
                base_time,
                base_time,
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate {
                    deadline: Some((base_time - Days::new(1)).date_naive()),
                    ..Default::default()
                },
                None,
                None,
                base_time,
                base_time,
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate {
                    deadline: Some(base_time.date_naive()),
                    ..Default::default()
                },
                None,
                None,
                base_time,
                base_time,
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate {
                    deadline: Some((base_time + Days::new(1)).date_naive()),
                    ..Default::default()
                },
                None,
                None,
                base_time,
                base_time,
            )
            .await,
        ];

        let opts = TaskQueryOptions {
            deadline_filter: Some(DateFilter::On(base_time.date_naive())),
            ..Default::default()
        };

        let res = query_tasks_inner(&mut tx, Uuid::nil(), opts).await;
        assert!(res.is_ok());
        if let Ok(tasks) = res {
            assert!(!tasks.is_empty(), "must have data to test on");

            assert!(
                tasks.iter().all(|task| if let Some(date) = task.deadline {
                    date == base_time.date_naive()
                } else {
                    false
                }),
                "should only return tasks that have deadline on given date"
            )
        }

        let opts = TaskQueryOptions {
            deadline_filter: Some(DateFilter::NotOn(base_time.date_naive())),
            ..Default::default()
        };

        let res = query_tasks_inner(&mut tx, Uuid::nil(), opts).await;
        assert!(res.is_ok());
        if let Ok(tasks) = res {
            assert!(!tasks.is_empty(), "must have data to test on");

            assert!(
                tasks.iter().all(|task| if let Some(date) = task.deadline {
                    date != base_time.date_naive()
                } else {
                    true
                }),
                "should only return tasks that have deadline not on given date"
            )
        }
    }

    #[test]
    async fn filter_deadline_after_excl() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = get_time();

        // create test data
        let _test_tasks = [
            create_test_task(
                &mut tx,
                TaskCreate::default(),
                None,
                None,
                base_time,
                base_time,
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate {
                    deadline: Some((base_time - Days::new(1)).date_naive()),
                    ..Default::default()
                },
                None,
                None,
                base_time,
                base_time,
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate {
                    deadline: Some(base_time.date_naive()),
                    ..Default::default()
                },
                None,
                None,
                base_time,
                base_time,
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate {
                    deadline: Some((base_time + Days::new(1)).date_naive()),
                    ..Default::default()
                },
                None,
                None,
                base_time,
                base_time,
            )
            .await,
        ];

        let opts = TaskQueryOptions {
            deadline_filter: Some(DateFilter::StartRange(DateBound::Exclusive(
                base_time.date_naive(),
            ))),
            ..Default::default()
        };

        let res = query_tasks_inner(&mut tx, Uuid::nil(), opts).await;
        assert!(res.is_ok());
        if let Ok(tasks) = res {
            assert!(!tasks.is_empty(), "must have data to test on");

            assert!(
                tasks.iter().all(|task| if let Some(date) = task.deadline {
                    date > base_time.date_naive()
                } else {
                    false
                }),
                "should only return tasks that have deadline after a given date excluding the given date"
            )
        }
    }

    #[test]
    async fn filter_deadline_after_incl() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = get_time();

        // create test data
        let _test_tasks = [
            create_test_task(
                &mut tx,
                TaskCreate::default(),
                None,
                None,
                base_time,
                base_time,
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate {
                    deadline: Some((base_time - Days::new(1)).date_naive()),
                    ..Default::default()
                },
                None,
                None,
                base_time,
                base_time,
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate {
                    deadline: Some(base_time.date_naive()),
                    ..Default::default()
                },
                None,
                None,
                base_time,
                base_time,
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate {
                    deadline: Some((base_time + Days::new(1)).date_naive()),
                    ..Default::default()
                },
                None,
                None,
                base_time,
                base_time,
            )
            .await,
        ];

        let opts = TaskQueryOptions {
            deadline_filter: Some(DateFilter::StartRange(DateBound::Inclusive(
                base_time.date_naive(),
            ))),
            ..Default::default()
        };

        let res = query_tasks_inner(&mut tx, Uuid::nil(), opts).await;
        assert!(res.is_ok());
        if let Ok(tasks) = res {
            assert!(!tasks.is_empty(), "must have data to test on");

            assert!(
                tasks.iter().all(|task| if let Some(date) = task.deadline {
                    date >= base_time.date_naive()
                } else {
                    false
                }),
                "should only return tasks that have deadline after a given date including the given date"
            )
        }
    }

    #[test]
    async fn filter_deadline_before_excl() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = get_time();

        // create test data
        let _test_tasks = [
            create_test_task(
                &mut tx,
                TaskCreate::default(),
                None,
                None,
                base_time,
                base_time,
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate {
                    deadline: Some((base_time - Days::new(1)).date_naive()),
                    ..Default::default()
                },
                None,
                None,
                base_time,
                base_time,
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate {
                    deadline: Some(base_time.date_naive()),
                    ..Default::default()
                },
                None,
                None,
                base_time,
                base_time,
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate {
                    deadline: Some((base_time + Days::new(1)).date_naive()),
                    ..Default::default()
                },
                None,
                None,
                base_time,
                base_time,
            )
            .await,
        ];

        let opts = TaskQueryOptions {
            deadline_filter: Some(DateFilter::EndRange(DateBound::Exclusive(
                base_time.date_naive(),
            ))),
            ..Default::default()
        };

        let res = query_tasks_inner(&mut tx, Uuid::nil(), opts).await;
        assert!(res.is_ok());
        if let Ok(tasks) = res {
            assert!(!tasks.is_empty(), "must have data to test on");

            assert!(
                tasks.iter().all(|task| if let Some(date) = task.deadline {
                    date < base_time.date_naive()
                } else {
                    false
                }),
                "should only return tasks that have deadline before a given date excluding the given date"
            )
        }
    }

    #[test]
    async fn filter_deadline_before_incl() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = get_time();

        // create test data
        let _test_tasks = [
            create_test_task(
                &mut tx,
                TaskCreate::default(),
                None,
                None,
                base_time,
                base_time,
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate {
                    deadline: Some((base_time - Days::new(1)).date_naive()),
                    ..Default::default()
                },
                None,
                None,
                base_time,
                base_time,
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate {
                    deadline: Some(base_time.date_naive()),
                    ..Default::default()
                },
                None,
                None,
                base_time,
                base_time,
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate {
                    deadline: Some((base_time + Days::new(1)).date_naive()),
                    ..Default::default()
                },
                None,
                None,
                base_time,
                base_time,
            )
            .await,
        ];

        let opts = TaskQueryOptions {
            deadline_filter: Some(DateFilter::EndRange(DateBound::Inclusive(
                base_time.date_naive(),
            ))),
            ..Default::default()
        };

        let res = query_tasks_inner(&mut tx, Uuid::nil(), opts).await;
        assert!(res.is_ok());
        if let Ok(tasks) = res {
            assert!(!tasks.is_empty(), "must have data to test on");

            assert!(
                tasks.iter().all(|task| if let Some(date) = task.deadline {
                    date <= base_time.date_naive()
                } else {
                    false
                }),
                "should only return tasks that have deadline before a given date including the given date"
            )
        }
    }

    #[test]
    async fn filter_deadline_between() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = get_time();

        // create test data
        let _test_tasks = [
            create_test_task(
                &mut tx,
                TaskCreate::default(),
                None,
                None,
                base_time,
                base_time,
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate {
                    deadline: Some((base_time - Days::new(2)).date_naive()),
                    ..Default::default()
                },
                None,
                None,
                base_time,
                base_time,
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate {
                    deadline: Some((base_time - Days::new(1)).date_naive()),
                    ..Default::default()
                },
                None,
                None,
                base_time,
                base_time,
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate {
                    deadline: Some(base_time.date_naive()),
                    ..Default::default()
                },
                None,
                None,
                base_time,
                base_time,
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate {
                    deadline: Some((base_time + Days::new(1)).date_naive()),
                    ..Default::default()
                },
                None,
                None,
                base_time,
                base_time,
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate {
                    deadline: Some((base_time + Days::new(2)).date_naive()),
                    ..Default::default()
                },
                None,
                None,
                base_time,
                base_time,
            )
            .await,
        ];

        let opts = TaskQueryOptions {
            deadline_filter: Some(DateFilter::Range(
                DateBound::Exclusive((base_time - Days::new(1)).date_naive()),
                DateBound::Exclusive((base_time + Days::new(1)).date_naive()),
            )),
            ..Default::default()
        };

        let res = query_tasks_inner(&mut tx, Uuid::nil(), opts).await;
        assert!(res.is_ok());
        if let Ok(tasks) = res {
            assert!(!tasks.is_empty(), "must have data to test on");

            assert!(
                tasks.iter().all(|task| if let Some(date) = task.deadline {
                    date > (base_time - Days::new(1)).date_naive()
                        && date < (base_time + Days::new(1)).date_naive()
                } else {
                    false
                }),
                "should only return tasks that have deadline between given range excluding both endpoints"
            )
        }

        let opts = TaskQueryOptions {
            deadline_filter: Some(DateFilter::Range(
                DateBound::Exclusive((base_time - Days::new(1)).date_naive()),
                DateBound::Inclusive((base_time + Days::new(1)).date_naive()),
            )),
            ..Default::default()
        };

        let res = query_tasks_inner(&mut tx, Uuid::nil(), opts).await;
        assert!(res.is_ok());
        if let Ok(tasks) = res {
            assert!(!tasks.is_empty(), "must have data to test on");

            assert!(
                tasks.iter().all(|task| if let Some(date) = task.deadline {
                    date > (base_time - Days::new(1)).date_naive()
                        && date <= (base_time + Days::new(1)).date_naive()
                } else {
                    false
                }),
                "should only return tasks that have deadline between given range excluding only start endpoint"
            )
        }

        let opts = TaskQueryOptions {
            deadline_filter: Some(DateFilter::Range(
                DateBound::Inclusive((base_time - Days::new(1)).date_naive()),
                DateBound::Inclusive((base_time + Days::new(1)).date_naive()),
            )),
            ..Default::default()
        };

        let res = query_tasks_inner(&mut tx, Uuid::nil(), opts).await;
        assert!(res.is_ok());
        if let Ok(tasks) = res {
            assert!(!tasks.is_empty(), "must have data to test on");

            assert!(
                tasks.iter().all(|task| if let Some(date) = task.deadline {
                    date >= (base_time - Days::new(1)).date_naive()
                        && date <= (base_time + Days::new(1)).date_naive()
                } else {
                    false
                }),
                "should only return tasks that have deadline between given range including both endpoints"
            )
        }

        let opts = TaskQueryOptions {
            deadline_filter: Some(DateFilter::Range(
                DateBound::Inclusive((base_time - Days::new(1)).date_naive()),
                DateBound::Exclusive((base_time + Days::new(1)).date_naive()),
            )),
            ..Default::default()
        };

        let res = query_tasks_inner(&mut tx, Uuid::nil(), opts).await;
        assert!(res.is_ok());
        if let Ok(tasks) = res {
            assert!(!tasks.is_empty(), "must have data to test on");

            assert!(
                tasks.iter().all(|task| if let Some(date) = task.deadline {
                    date >= (base_time - Days::new(1)).date_naive()
                        && date < (base_time + Days::new(1)).date_naive()
                } else {
                    false
                }),
                "should only return tasks that have deadline between given range excluding only end endpoint"
            )
        }
    }

    #[test]
    async fn query_returns_tasks_with_tags() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = get_time();

        // create test data
        let _test_tags = [
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await,
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await,
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await,
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await,
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await,
        ];
        let _test_tasks = [
            create_test_task(
                &mut tx,
                TaskCreate {
                    title: "0".to_string(),
                    ..Default::default()
                },
                None,
                None,
                base_time,
                base_time,
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate {
                    title: "1".to_string(),
                    tags: _test_tags[..1].iter().map(|tag| tag.id).collect(),
                    ..Default::default()
                },
                None,
                None,
                base_time,
                base_time,
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate {
                    title: "2".to_string(),
                    tags: _test_tags[..2].iter().map(|tag| tag.id).collect(),
                    ..Default::default()
                },
                None,
                None,
                base_time,
                base_time,
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate {
                    title: "3".to_string(),
                    tags: _test_tags[..3].iter().map(|tag| tag.id).collect(),
                    ..Default::default()
                },
                None,
                None,
                base_time,
                base_time,
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate {
                    title: "4".to_string(),
                    tags: _test_tags[..4].iter().map(|tag| tag.id).collect(),
                    ..Default::default()
                },
                None,
                None,
                base_time,
                base_time,
            )
            .await,
            create_test_task(
                &mut tx,
                TaskCreate {
                    title: "5".to_string(),
                    tags: _test_tags[..5].iter().map(|tag| tag.id).collect(),
                    ..Default::default()
                },
                None,
                None,
                base_time,
                base_time,
            )
            .await,
        ];

        let opts = TaskQueryOptions {
            limit: Some(MAX_LIMIT),
            ..Default::default()
        };

        let res = query_tasks_inner(&mut tx, Uuid::nil(), opts).await;
        assert!(res.is_ok());
        if let Ok(tasks) = res {
            assert!(!tasks.is_empty(), "must have data to test on");

            for task in tasks {
                let num_tags: usize = task.title.parse().unwrap();

                assert_eq!(task.tags.len(), num_tags);
            }
        }
    }

    #[test]
    async fn as_nonexistent_user() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let opts = TaskQueryOptions::default();

        let res = query_tasks_inner(&mut tx, Uuid::new_v4(), opts).await;
        assert!(res.is_ok());
        if let Ok(tasks) = res {
            assert!(tasks.is_empty());
        }
    }

    #[test]
    async fn full_combination() {
        // Use all filters

        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = get_time();

        let opts = TaskQueryOptions {
            limit: Some(5),
            offset: Some(10),
            sort_order: TaskSort::Created(SortOrder::Ascending),
            completed: true,
            deleted: true,
            start_filter: Some(DateFilter::Range(
                DateBound::Exclusive(base_time.date_naive()),
                DateBound::Inclusive((base_time + Days::new(5)).date_naive()),
            )),
            deadline_filter: Some(DateFilter::Range(
                DateBound::Exclusive(base_time.date_naive()),
                DateBound::Inclusive((base_time + Days::new(5)).date_naive()),
            )),
        };

        assert!(query_tasks_inner(&mut tx, Uuid::nil(), opts).await.is_ok());
    }
}

#[cfg(test)]
mod select_tests {
    use std::{collections::HashSet, time::Duration};

    use tokio::test;
    use uuid::Uuid;

    use super::select_task_inner;
    use crate::{
        db::test_utils::{create_test_tag, create_test_task, get_pool, get_time},
        routes::models::{tag::Model as TagCreate, task::Model as TaskCreate},
    };

    #[test]
    async fn base_select() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = get_time();

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
    async fn many_various_tasks() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = get_time();

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
    async fn task_with_tag() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = get_time();

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
    async fn task_with_multiple_tags() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = get_time();

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
    async fn deleted_task() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = get_time();

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
    async fn nonexistent_task() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let res = select_task_inner(&mut tx, Uuid::new_v4(), Uuid::nil()).await;
        assert!(res.is_ok());

        let task_opt = res.unwrap();
        assert!(task_opt.is_none());
    }

    #[test]
    async fn as_nonexistent_user() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = get_time();

        let task = create_test_task(
            &mut tx,
            TaskCreate::default(),
            None,
            None,
            base_time,
            base_time + Duration::from_hours(1),
        )
        .await;

        let res = select_task_inner(&mut tx, task.id, Uuid::new_v4()).await;
        assert!(res.is_ok());

        let task_opt = res.unwrap();
        assert!(task_opt.is_none());
    }
}

#[cfg(test)]
mod insert_tests {
    use chrono::{Duration, Local, NaiveDate, NaiveDateTime, NaiveTime};
    use tokio::test;
    use uuid::Uuid;

    use super::insert_task_inner;
    use crate::db::test_utils::{create_test_tag, get_pool, get_time};
    use crate::{
        db::{ApplicationError, Error},
        routes::models::{Start, tag::Model as TagCreate, task::Model as TaskCreate},
    };

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

        let date = get_time().date_naive();

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

        let datetime = get_time();

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

        let date = get_time().date_naive();

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

        let base_time = get_time();

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

        let base_time = get_time();

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
    async fn as_nonexistent_user() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let res = insert_task_inner(&mut tx, Uuid::new_v4(), TaskCreate::default()).await;
        assert!(res.is_err());
        if let Err(err) = res {
            assert!(matches!(
                err,
                Error::Application(ApplicationError::UserNotFound)
            ))
        }
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
        assert_eq!(task.start_dt.unwrap(), start_datetime);
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
    use chrono::{Duration, NaiveDate};
    use tokio::test;
    use uuid::Uuid;

    use super::update_task_inner;
    use crate::db::test_utils::{create_test_tag, create_test_task, get_pool, get_task, get_time};
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
    async fn is_idempotent() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = get_time();

        let before_task = create_test_task(
            &mut tx,
            TaskCreate::default(),
            None,
            None,
            base_time,
            base_time,
        )
        .await;

        let task_update: TaskCreate = before_task.clone().into();

        let res = update_task_inner(&mut tx, before_task.id, Uuid::nil(), task_update).await;
        assert!(res.is_ok());

        let after_task = res.unwrap();
        assert_eq!(after_task, before_task);
    }

    #[test]
    async fn title_only() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = get_time();

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

        let base_time = get_time();

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

        let base_time = get_time();

        let before_task = create_test_task(
            &mut tx,
            TaskCreate::default(),
            None,
            None,
            base_time,
            base_time,
        )
        .await;

        let updated_start = get_time().date_naive();
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

        let base_time = get_time();

        let before_task = create_test_task(
            &mut tx,
            TaskCreate::default(),
            None,
            None,
            base_time,
            base_time,
        )
        .await;

        let updated_start = get_time();
        let mut updated_task: TaskCreate = before_task.clone().into();
        updated_task.start = Some(Start::At(updated_start));

        let res = update_task_inner(&mut tx, before_task.id, Uuid::nil(), updated_task).await;
        assert!(res.is_ok());

        let after_task = res.unwrap();
        assert_ne!(before_task.updated_at, after_task.updated_at);
        assert_ne!(before_task.start_dt, after_task.start_dt);
        assert_ne!(before_task.has_time, after_task.has_time);
        assert_eq!(after_task.start_dt.unwrap(), updated_start);

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

        // let base_time = get_time();

        // let datetime = get_time();
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

        let base_time = get_time();

        let before_task = create_test_task(
            &mut tx,
            TaskCreate::default(),
            None,
            None,
            base_time,
            base_time,
        )
        .await;

        let updated_deadline = get_time().date_naive();
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

        let base_time = get_time();

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

        let base_time = get_time();

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

        let base_time = get_time();

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

        let base_time = get_time();

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

        let base_time = get_time();

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
    async fn as_nonexistent_user() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = get_time();

        let task = create_test_task(
            &mut tx,
            TaskCreate::default(),
            None,
            None,
            base_time,
            base_time,
        )
        .await;

        let res = update_task_inner(&mut tx, task.id, Uuid::new_v4(), task.clone().into()).await;
        assert!(res.is_err());
        if let Err(err) = res {
            assert!(matches!(
                err,
                Error::Application(ApplicationError::TaskNotFound)
            ))
        }
    }

    #[test]
    async fn combination_1() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = get_time();

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

        let base_time = get_time();

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

        let base_time = get_time();

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
    use tokio::test;
    use uuid::Uuid;

    use super::delete_task_inner;
    use crate::db::test_utils::{create_test_task, get_pool, get_task, get_time};
    use crate::routes::models::task::Model as TaskCreate;

    #[test]
    async fn base_delete() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = get_time();

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
    async fn is_idempotent() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = get_time();

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
    async fn deleted_task() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = get_time();

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
    async fn nonexistent_task() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let res = delete_task_inner(&mut tx, Uuid::new_v4(), Uuid::nil()).await;
        assert!(res.is_ok());

        assert_eq!(res.unwrap(), (), "delete should return unit '()'");
    }

    #[test]
    async fn as_nonexistent_user() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = get_time();

        let task = create_test_task(
            &mut tx,
            TaskCreate::default(),
            None,
            None,
            base_time,
            base_time,
        )
        .await;

        let res = delete_task_inner(&mut tx, task.id, Uuid::new_v4()).await;
        assert!(res.is_ok());
    }
}

#[cfg(test)]
mod restore_tests {
    use tokio::test;
    use uuid::Uuid;

    use super::restore_task_inner;
    use crate::db::test_utils::{create_test_task, get_pool, get_task, get_time};
    use crate::{
        db::{ApplicationError, Error},
        routes::models::task::Model as TaskCreate,
    };

    #[test]
    async fn base_restore() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = get_time();

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
    async fn is_idempotent() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = get_time();

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
    async fn restored_task() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = get_time();

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
    async fn nonexistent_task() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let res = restore_task_inner(&mut tx, Uuid::new_v4(), Uuid::nil()).await;
        assert!(matches!(
            res,
            Err(Error::Application(ApplicationError::TaskNotFound))
        ));
    }

    #[test]
    async fn as_nonexistent_user() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = get_time();

        let task = create_test_task(
            &mut tx,
            TaskCreate::default(),
            None,
            None,
            base_time,
            base_time,
        )
        .await;

        let res = restore_task_inner(&mut tx, task.id, Uuid::new_v4()).await;
        assert!(res.is_err());
        if let Err(err) = res {
            assert!(matches!(
                err,
                Error::Application(ApplicationError::TaskNotFound)
            ))
        }
    }
}

#[cfg(test)]
mod complete_tests {
    use tokio::test;
    use uuid::Uuid;

    use super::complete_task_inner;
    use crate::db::test_utils::{create_test_task, get_pool, get_task, get_time};
    use crate::{
        db::{ApplicationError, Error},
        routes::models::task::Model as TaskCreate,
    };

    #[test]
    async fn complete_task() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = get_time();

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
    async fn complete_is_idempotent() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = get_time();

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
    async fn complete_completed() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = get_time();

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
    async fn complete_deleted() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = get_time();

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
    async fn complete_as_nonexistent_user() {
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

        let base_time = get_time();

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
    async fn uncomplete_is_idempotent() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = get_time();

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
    async fn uncomplete_uncompleted() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = get_time();

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
    async fn uncomplete_deleted() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = get_time();

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

    #[test]
    async fn uncomplete_as_nonexistent_user() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let res = complete_task_inner(&mut tx, Uuid::new_v4(), Uuid::nil(), false).await;
        assert!(matches!(
            res,
            Err(Error::Application(ApplicationError::TaskNotFound))
        ));
    }
}
