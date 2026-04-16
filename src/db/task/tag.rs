//! # Task Tag Database Functions
//!
//! This module contains database functions for tags associated with tasks

use std::collections::HashSet;

use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use super::select_task_inner;
use crate::{
    db::{Error, Result, error::ApplicationError, query_as_wrapper},
    models::Tag,
};

/// Query database for all tags associated with a task
///
/// # Arguments
///
/// * `pool`: Database connection pool
/// * `task_id`: ID of task to query tags for
/// * `user_id`: User ID of task owner
///
/// # Returns
///
/// List of tags associated with task `task_id`
pub async fn query_task_tags(pool: PgPool, task_id: Uuid, user_id: Uuid) -> Result<Vec<Tag>> {
    let mut conn = pool.acquire().await?;
    let tags = query_task_tags_inner(&mut conn, task_id, user_id).await?;
    conn.close().await?;

    Ok(tags)
}

/// Internal function for `query_task_tags`
///
/// Only used internally
async fn query_task_tags_inner(
    conn: &mut PgConnection,
    task_id: Uuid,
    user_id: Uuid,
) -> Result<Vec<Tag>> {
    if select_task_inner(conn, task_id, user_id).await?.is_none() {
        return Err(Error::Application(ApplicationError::TaskNotFound));
    }

    Ok(query_as_wrapper::<Tag>(
        "SELECT tg.*
                FROM app.tags tg
                JOIN app.task_tags tt ON tg.id = tt.tag_id
                JOIN app.tasks t ON tt.task_id = t.id
                WHERE tt.task_id = $1 AND t.created_by = $2
                ORDER BY tg.label ASC, updated_at DESC",
    )
    .bind(task_id)
    .bind(user_id)
    .fetch_all(conn)
    .await?)
}

/// Add tags to a task
///
/// This function adds tasks in addition to the existing tags in the database
///
/// # Arguments
///
/// * `pool`: Database connection pool
/// * `task_id`: ID of task to add tag to
/// * `user_id`: User ID of task owner
/// * `tag_ids`: ID of tags to add to task
///
/// # Returns
///
/// Unit `()`
pub async fn insert_task_tags(
    pool: PgPool,
    task_id: Uuid,
    user_id: Uuid,
    tag_ids: Vec<Uuid>,
) -> Result<()> {
    let mut tx = pool.begin().await?;
    insert_task_tags_inner(&mut tx, task_id, user_id, tag_ids).await?;
    tx.commit().await?;

    Ok(())
}

/// Internal function for `insert_task_tag`
///
/// Only used internally
async fn insert_task_tags_inner(
    conn: &mut PgConnection,
    task_id: Uuid,
    user_id: Uuid,
    tag_ids: Vec<Uuid>,
) -> Result<()> {
    if select_task_inner(conn, task_id, user_id).await?.is_none() {
        return Err(Error::Application(ApplicationError::TaskNotFound));
    }

    let res = sqlx::query(
        "INSERT INTO app.task_tags (task_id, tag_id)
        SELECT $1, unnest_tag
        FROM UNNEST($2) AS unnest_tag
        ON CONFLICT DO NOTHING",
    )
    .bind(task_id)
    .bind(tag_ids)
    .execute(conn.as_mut())
    .await;

    let inserted = match res {
        Ok(done) => done.rows_affected(),
        Err(err) => {
            if let Some(db_err) = err.as_database_error()
                && let Some("task_tags_tag_id_fkey") = db_err.constraint()
            {
                return Err(Error::Application(ApplicationError::TagNotFound));
            }

            return Err(err.into());
        }
    };

    if inserted > 0 {
        sqlx::query(
            "UPDATE app.tasks SET
            updated_at = CURRENT_TIMESTAMP
            WHERE id = $1 AND created_by = $2",
        )
        .bind(task_id)
        .bind(user_id)
        .execute(conn.as_mut())
        .await?;
    }

    Ok(())
}

/// Update all tags associated with a task
///
/// This function replaces the set of tags in the database
///
/// # Arguments
///
/// * `pool`: Database connection pool
/// * `task_id`: ID of task to update tags for
/// * `user_id`: User ID of task owner
/// * `tag_ids`: ID of tags to update task with
///
/// # Returns
///
/// Unit `()`
pub async fn update_task_tags(
    pool: PgPool,
    task_id: Uuid,
    user_id: Uuid,
    tag_ids: Vec<Uuid>,
) -> Result<()> {
    let mut tx = pool.begin().await?;
    update_task_tags_inner(&mut tx, task_id, user_id, tag_ids).await?;
    tx.commit().await?;

    Ok(())
}

/// Internal function for `update_task_tags`
///
/// Only used internally
pub(super) async fn update_task_tags_inner(
    conn: &mut PgConnection,
    task_id: Uuid,
    user_id: Uuid,
    tag_ids: Vec<Uuid>,
) -> Result<Vec<Tag>> {
    if select_task_inner(conn, task_id, user_id).await?.is_none() {
        return Err(Error::Application(ApplicationError::TaskNotFound));
    }

    // TODO: change this process
    // 1. Get existing tags
    // 2. Get difference between the set: 'to_remove' list and 'to_add' list
    // 3. Insert 'to_add' tags
    // 4. Remove 'to_remove' tags

    // check if new set is the same; if it is, stop here
    let current_tag_ids: Vec<Uuid> =
        sqlx::query_scalar("SELECT tag_id FROM app.task_tags WHERE task_id = $1")
            .bind(task_id)
            .fetch_all(conn.as_mut())
            .await?;

    let current: HashSet<Uuid> = current_tag_ids.into_iter().collect();
    let desired: HashSet<Uuid> = tag_ids.iter().copied().collect();

    if current == desired {
        return query_task_tags_inner(conn, task_id, user_id).await;
    }

    let to_remove: Vec<Uuid> = current.difference(&desired).copied().collect();
    if !to_remove.is_empty() {
        sqlx::query("DELETE FROM app.task_tags WHERE task_id = $1 AND tag_id = ANY($2)")
            .bind(task_id)
            .bind(to_remove)
            .execute(conn.as_mut())
            .await?;
    }

    let to_add: Vec<Uuid> = desired.difference(&current).copied().collect();
    if !to_add.is_empty()
        && let Err(err) = sqlx::query(
            "INSERT INTO app.task_tags (task_id, tag_id)
            SELECT $1, unnest_tag
            FROM UNNEST($2::uuid[]) AS unnest_tag",
        )
        .bind(task_id)
        .bind(to_add)
        .execute(conn.as_mut())
        .await
    {
        if let Some(db_err) = err.as_database_error()
            && let Some("task_tags_tag_id_fkey") = db_err.constraint()
        {
            return Err(Error::Application(ApplicationError::TagNotFound));
        }

        return Err(err.into());
    }

    sqlx::query(
        "UPDATE app.tasks SET
            updated_at = CURRENT_TIMESTAMP
            WHERE id = $1 AND created_by = $2",
    )
    .bind(task_id)
    .bind(user_id)
    .execute(conn.as_mut())
    .await?;

    query_task_tags_inner(conn, task_id, user_id).await
}

/// Remove a tag from a task
///
/// # Arguments
///
/// * `pool`: Database connection pool
/// * `task_id`: ID of task to delete tag from
/// * `user_id`: User ID of task owner
/// * `tag_id`: ID of tag to add to task
///
/// # Returns
///
/// Unit `()`
pub async fn delete_task_tag(
    pool: PgPool,
    task_id: Uuid,
    user_id: Uuid,
    tag_id: Uuid,
) -> Result<()> {
    let mut tx = pool.begin().await?;
    delete_task_tag_inner(&mut tx, task_id, user_id, tag_id).await?;
    tx.commit().await?;

    Ok(())
}

/// Internal function for `delete_task_tag`
///
/// Only used internally
pub async fn delete_task_tag_inner(
    conn: &mut PgConnection,
    task_id: Uuid,
    user_id: Uuid,
    tag_id: Uuid,
) -> Result<()> {
    if select_task_inner(conn, task_id, user_id).await?.is_none() {
        return Err(Error::Application(ApplicationError::TaskNotFound));
    }

    let deleted = sqlx::query("DELETE FROM app.task_tags WHERE task_id = $1 AND tag_id = $2")
        .bind(task_id)
        .bind(tag_id)
        .execute(conn.as_mut())
        .await?
        .rows_affected();

    if deleted > 0 {
        sqlx::query(
            "UPDATE app.tasks SET
            updated_at = CURRENT_TIMESTAMP
            WHERE id = $1 AND created_by = $2",
        )
        .bind(task_id)
        .bind(user_id)
        .execute(conn.as_mut())
        .await?;
    }

    Ok(())
}

#[cfg(test)]
mod query_tests {
    use tokio::test;
    use uuid::Uuid;

    use super::query_task_tags_inner;
    use crate::db::test_utils::{create_test_tag, create_test_task, get_pool, get_time};
    use crate::{
        db::{ApplicationError, Error},
        models::Tag,
        routes::models::{tag::Model as TagCreate, task::Model as TaskCreate},
    };

    // async fn create_test_data(tx: &mut PgConnection) {
    //     let mut num_tasks = 0;
    //     let mut num_tags = 0;

    //     // Create
    // }

    #[test]
    async fn base_query() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = get_time();

        let tag_list = [
            create_test_tag(
                &mut tx,
                TagCreate {
                    label: "Tag 1".to_string(),
                    ..Default::default()
                },
                base_time,
                base_time,
            )
            .await,
            create_test_tag(
                &mut tx,
                TagCreate {
                    label: "Tag 2".to_string(),
                    ..Default::default()
                },
                base_time,
                base_time,
            )
            .await,
            create_test_tag(
                &mut tx,
                TagCreate {
                    label: "Tag 3".to_string(),
                    ..Default::default()
                },
                base_time,
                base_time,
            )
            .await,
        ];

        let task = create_test_task(
            &mut tx,
            TaskCreate {
                tags: tag_list.iter().map(|tag| tag.id).collect(),
                ..Default::default()
            },
            None,
            None,
            base_time,
            base_time,
        )
        .await;

        let res = query_task_tags_inner(&mut tx, task.id, Uuid::nil()).await;
        assert!(res.is_ok());

        let task_tags = res.unwrap();
        assert_eq!(task_tags.len(), tag_list.len());
        for task_tag in task_tags {
            assert!(matches!(task_tag, Tag { .. }));
        }
    }

    #[test]
    async fn no_tags() {
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

        let res = query_task_tags_inner(&mut tx, task.id, Uuid::nil()).await;
        assert!(res.is_ok());

        let task_tags = res.unwrap();
        assert_eq!(task_tags.len(), 0);
    }

    #[test]
    async fn one_tag() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = get_time();

        let tag = create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await;

        let task = create_test_task(
            &mut tx,
            TaskCreate {
                tags: vec![tag.id],
                ..Default::default()
            },
            None,
            None,
            base_time,
            base_time,
        )
        .await;

        let res = query_task_tags_inner(&mut tx, task.id, Uuid::nil()).await;
        assert!(res.is_ok());

        let task_tags = res.unwrap();
        assert_eq!(task_tags.len(), 1);
        assert_eq!(task_tags[0], tag);
    }

    #[test]
    async fn many_tags() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = get_time();

        let tag_list = [
            create_test_tag(
                &mut tx,
                TagCreate {
                    label: "Tag 1".to_string(),
                    ..Default::default()
                },
                base_time,
                base_time,
            )
            .await,
            create_test_tag(
                &mut tx,
                TagCreate {
                    label: "Tag 2".to_string(),
                    ..Default::default()
                },
                base_time,
                base_time,
            )
            .await,
            create_test_tag(
                &mut tx,
                TagCreate {
                    label: "Tag 3".to_string(),
                    ..Default::default()
                },
                base_time,
                base_time,
            )
            .await,
        ];

        let task = create_test_task(
            &mut tx,
            TaskCreate {
                tags: tag_list.iter().map(|tag| tag.id).collect(),
                ..Default::default()
            },
            None,
            None,
            base_time,
            base_time,
        )
        .await;

        let res = query_task_tags_inner(&mut tx, task.id, Uuid::nil()).await;
        assert!(res.is_ok());

        let task_tags = res.unwrap();
        assert_eq!(task_tags.len(), tag_list.len());
        for (task_tag, original_tag) in task_tags.iter().zip(tag_list.iter()) {
            assert_eq!(task_tag, original_tag);
        }
    }

    #[test]
    async fn deleted_task() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = get_time();

        // No Tags
        let task = create_test_task(
            &mut tx,
            TaskCreate::default(),
            None,
            Some(base_time),
            base_time,
            base_time,
        )
        .await;

        let res = query_task_tags_inner(&mut tx, task.id, Uuid::nil()).await;
        assert!(res.is_err());
        if let Err(err) = res {
            assert!(matches!(
                err,
                Error::Application(ApplicationError::TaskNotFound)
            ))
        }

        // One Tag
        let tag = create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await;

        let task = create_test_task(
            &mut tx,
            TaskCreate {
                tags: vec![tag.id],
                ..Default::default()
            },
            None,
            Some(base_time),
            base_time,
            base_time,
        )
        .await;

        let res = query_task_tags_inner(&mut tx, task.id, Uuid::nil()).await;
        assert!(res.is_err());
        if let Err(err) = res {
            assert!(matches!(
                err,
                Error::Application(ApplicationError::TaskNotFound)
            ))
        }

        // Many Tags
        let tag_list = [
            create_test_tag(
                &mut tx,
                TagCreate {
                    label: "Tag 1".to_string(),
                    ..Default::default()
                },
                base_time,
                base_time,
            )
            .await,
            create_test_tag(
                &mut tx,
                TagCreate {
                    label: "Tag 2".to_string(),
                    ..Default::default()
                },
                base_time,
                base_time,
            )
            .await,
            create_test_tag(
                &mut tx,
                TagCreate {
                    label: "Tag 3".to_string(),
                    ..Default::default()
                },
                base_time,
                base_time,
            )
            .await,
        ];

        let task = create_test_task(
            &mut tx,
            TaskCreate {
                tags: tag_list.iter().map(|tag| tag.id).collect(),
                ..Default::default()
            },
            None,
            Some(base_time),
            base_time,
            base_time,
        )
        .await;

        let res = query_task_tags_inner(&mut tx, task.id, Uuid::nil()).await;
        assert!(res.is_err());
        if let Err(err) = res {
            assert!(matches!(
                err,
                Error::Application(ApplicationError::TaskNotFound)
            ))
        }
    }

    #[test]
    async fn nonexistent_task() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let res = query_task_tags_inner(&mut tx, Uuid::new_v4(), Uuid::nil()).await;
        assert!(res.is_err());
        if let Err(err) = res {
            assert!(matches!(
                err,
                Error::Application(ApplicationError::TaskNotFound)
            ))
        }
    }

    #[test]
    async fn as_nonexistent_user() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = get_time();

        // No Tags
        let task = create_test_task(
            &mut tx,
            TaskCreate::default(),
            None,
            Some(base_time),
            base_time,
            base_time,
        )
        .await;

        let res = query_task_tags_inner(&mut tx, task.id, Uuid::new_v4()).await;
        assert!(res.is_err());
        if let Err(err) = res {
            assert!(matches!(
                err,
                Error::Application(ApplicationError::TaskNotFound)
            ))
        }

        // One Tag
        let tag = create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await;

        let task = create_test_task(
            &mut tx,
            TaskCreate {
                tags: vec![tag.id],
                ..Default::default()
            },
            None,
            Some(base_time),
            base_time,
            base_time,
        )
        .await;

        let res = query_task_tags_inner(&mut tx, task.id, Uuid::new_v4()).await;
        assert!(res.is_err());
        if let Err(err) = res {
            assert!(matches!(
                err,
                Error::Application(ApplicationError::TaskNotFound)
            ))
        }

        // Many Tags
        let tag_list = [
            create_test_tag(
                &mut tx,
                TagCreate {
                    label: "Tag 1".to_string(),
                    ..Default::default()
                },
                base_time,
                base_time,
            )
            .await,
            create_test_tag(
                &mut tx,
                TagCreate {
                    label: "Tag 2".to_string(),
                    ..Default::default()
                },
                base_time,
                base_time,
            )
            .await,
            create_test_tag(
                &mut tx,
                TagCreate {
                    label: "Tag 3".to_string(),
                    ..Default::default()
                },
                base_time,
                base_time,
            )
            .await,
        ];

        let task = create_test_task(
            &mut tx,
            TaskCreate {
                tags: tag_list.iter().map(|tag| tag.id).collect(),
                ..Default::default()
            },
            None,
            Some(base_time),
            base_time,
            base_time,
        )
        .await;

        let res = query_task_tags_inner(&mut tx, task.id, Uuid::new_v4()).await;
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
mod insert_tests {
    use tokio::test;
    use uuid::Uuid;

    use crate::db::test_utils::{create_test_tag, create_test_task, get_pool, get_task, get_time};
    use crate::{
        db::{ApplicationError, Error, task::tag::insert_task_tags_inner},
        routes::models::{tag::Model as TagCreate, task::Model as TaskCreate},
    };

    #[test]
    async fn base_append() {
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

        let res = insert_task_tags_inner(&mut tx, task.id, Uuid::nil(), vec![]).await;
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), ());
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

        let tag_list = [
            create_test_tag(
                &mut tx,
                TagCreate {
                    label: "Tag 1".to_string(),
                    ..Default::default()
                },
                base_time,
                base_time,
            )
            .await,
            create_test_tag(
                &mut tx,
                TagCreate {
                    label: "Tag 2".to_string(),
                    ..Default::default()
                },
                base_time,
                base_time,
            )
            .await,
            create_test_tag(
                &mut tx,
                TagCreate {
                    label: "Tag 3".to_string(),
                    ..Default::default()
                },
                base_time,
                base_time,
            )
            .await,
        ];

        let res = insert_task_tags_inner(
            &mut tx,
            task.id,
            Uuid::nil(),
            tag_list.iter().map(|tag| tag.id).collect(),
        )
        .await;
        assert!(res.is_ok());

        let ret_task = get_task(&mut tx, task.id).await;
        assert_eq!(ret_task.tags, tag_list);

        let res = insert_task_tags_inner(
            &mut tx,
            task.id,
            Uuid::nil(),
            tag_list.iter().map(|tag| tag.id).collect(),
        )
        .await;
        assert!(res.is_ok());

        let ret_task = get_task(&mut tx, task.id).await;
        assert_eq!(ret_task.tags, tag_list);
    }

    #[test]
    async fn empty_list() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = get_time();

        // No existing tags
        let task = create_test_task(
            &mut tx,
            TaskCreate::default(),
            None,
            None,
            base_time,
            base_time,
        )
        .await;

        let res = insert_task_tags_inner(&mut tx, task.id, Uuid::nil(), vec![]).await;
        assert!(res.is_ok());

        let ret_task = get_task(&mut tx, task.id).await;
        assert!(ret_task.tags.is_empty());

        // With one existing tag
        let tag_list =
            vec![create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await];

        let task = create_test_task(
            &mut tx,
            TaskCreate {
                tags: tag_list.iter().map(|tag| tag.id).collect(),
                ..Default::default()
            },
            None,
            None,
            base_time,
            base_time,
        )
        .await;

        let res = insert_task_tags_inner(&mut tx, task.id, Uuid::nil(), vec![]).await;
        assert!(res.is_ok());

        let ret_task = get_task(&mut tx, task.id).await;
        assert_eq!(ret_task.tags, tag_list);

        // With many existing tags
        let tag_list = vec![
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await,
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await,
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await,
        ];

        let task = create_test_task(
            &mut tx,
            TaskCreate {
                tags: tag_list.iter().map(|tag| tag.id).collect(),
                ..Default::default()
            },
            None,
            None,
            base_time,
            base_time,
        )
        .await;

        let res = insert_task_tags_inner(&mut tx, task.id, Uuid::nil(), vec![]).await;
        assert!(res.is_ok());

        let ret_task = get_task(&mut tx, task.id).await;
        assert_eq!(ret_task.tags, tag_list);
    }

    #[test]
    async fn one_tag() {
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

        let mut tag_list =
            vec![create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await];
        let res = insert_task_tags_inner(&mut tx, task.id, Uuid::nil(), vec![tag_list[0].id]).await;
        assert!(res.is_ok());

        let ret_task = get_task(&mut tx, task.id).await;
        assert_eq!(ret_task.tags.len(), 1);
        assert_eq!(ret_task.tags, tag_list);

        tag_list.push(create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await);
        let res = insert_task_tags_inner(&mut tx, task.id, Uuid::nil(), vec![tag_list[1].id]).await;
        assert!(res.is_ok());

        let ret_task = get_task(&mut tx, task.id).await;
        assert_eq!(ret_task.tags.len(), 2);
        assert_eq!(ret_task.tags, tag_list);

        tag_list.push(create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await);
        let res = insert_task_tags_inner(&mut tx, task.id, Uuid::nil(), vec![tag_list[2].id]).await;
        assert!(res.is_ok());

        let ret_task = get_task(&mut tx, task.id).await;
        assert_eq!(ret_task.tags.len(), 3);
        assert_eq!(ret_task.tags, tag_list);
    }

    #[test]
    async fn many_tags() {
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

        let mut tag_list = vec![
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await,
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await,
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await,
        ];
        let res = insert_task_tags_inner(
            &mut tx,
            task.id,
            Uuid::nil(),
            tag_list[..3].iter().map(|tag| tag.id).collect(),
        )
        .await;
        assert!(res.is_ok());

        let ret_task = get_task(&mut tx, task.id).await;
        assert_eq!(ret_task.tags.len(), 3);
        assert_eq!(ret_task.tags, tag_list);

        tag_list.append(&mut vec![
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await,
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await,
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await,
        ]);
        let res = insert_task_tags_inner(
            &mut tx,
            task.id,
            Uuid::nil(),
            tag_list[3..6].iter().map(|tag| tag.id).collect(),
        )
        .await;
        assert!(res.is_ok());

        let ret_task = get_task(&mut tx, task.id).await;
        assert_eq!(ret_task.tags.len(), 6);
        assert_eq!(ret_task.tags, tag_list);

        tag_list.append(&mut vec![
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await,
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await,
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await,
        ]);
        let res = insert_task_tags_inner(
            &mut tx,
            task.id,
            Uuid::nil(),
            tag_list[6..9].iter().map(|tag| tag.id).collect(),
        )
        .await;
        assert!(res.is_ok());

        let ret_task = get_task(&mut tx, task.id).await;
        assert_eq!(ret_task.tags.len(), 9);
        assert_eq!(ret_task.tags, tag_list);

        let res = insert_task_tags_inner(
            &mut tx,
            task.id,
            Uuid::nil(),
            tag_list.iter().map(|tag| tag.id).collect(),
        )
        .await;
        assert!(res.is_ok());

        let ret_task = get_task(&mut tx, task.id).await;
        assert_eq!(ret_task.tags.len(), 9);
        assert_eq!(ret_task.tags, tag_list);
    }

    #[test]
    async fn nonexistent_tag_one() {
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

        let res = insert_task_tags_inner(&mut tx, task.id, Uuid::nil(), vec![Uuid::new_v4()]).await;
        assert!(res.is_err());
        if let Err(err) = res {
            assert!(matches!(
                err,
                Error::Application(ApplicationError::TagNotFound)
            ))
        }
    }

    #[test]
    async fn nonexistent_tag_within() {
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

        let tag_ids = vec![
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time)
                .await
                .id,
            Uuid::new_v4(),
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time)
                .await
                .id,
            Uuid::new_v4(),
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time)
                .await
                .id,
            Uuid::new_v4(),
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time)
                .await
                .id,
            Uuid::new_v4(),
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time)
                .await
                .id,
            Uuid::new_v4(),
        ];

        let res = insert_task_tags_inner(&mut tx, task.id, Uuid::nil(), tag_ids).await;
        assert!(res.is_err());
        if let Err(err) = res {
            assert!(matches!(
                err,
                Error::Application(ApplicationError::TagNotFound)
            ))
        }
    }

    #[test]
    async fn nonexistent_tag_all() {
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

        let tag_ids = vec![
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
        ];

        let res = insert_task_tags_inner(&mut tx, task.id, Uuid::nil(), tag_ids).await;
        assert!(res.is_err());
        if let Err(err) = res {
            assert!(matches!(
                err,
                Error::Application(ApplicationError::TagNotFound)
            ))
        }
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

        let res = insert_task_tags_inner(&mut tx, task.id, Uuid::nil(), vec![]).await;
        assert!(res.is_err());
        if let Err(err) = res {
            assert!(matches!(
                err,
                Error::Application(ApplicationError::TaskNotFound)
            ))
        }
    }

    #[test]
    async fn nonexistent_task() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let res = insert_task_tags_inner(&mut tx, Uuid::new_v4(), Uuid::nil(), vec![]).await;
        assert!(res.is_err());
        if let Err(err) = res {
            assert!(matches!(
                err,
                Error::Application(ApplicationError::TaskNotFound)
            ))
        }
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
            Some(base_time),
            base_time,
            base_time,
        )
        .await;

        let res = insert_task_tags_inner(&mut tx, task.id, Uuid::nil(), vec![]).await;
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
mod update_tests {
    use tokio::test;
    use uuid::Uuid;

    use super::update_task_tags_inner;
    use crate::db::test_utils::{create_test_tag, create_test_task, get_pool, get_task, get_time};
    use crate::db::{ApplicationError, Error};
    use crate::models::Tag;
    use crate::routes::models::{tag::Model as TagCreate, task::Model as TaskCreate};

    #[test]
    async fn base_update() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = get_time();

        let tag_list = [
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await,
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await,
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await,
        ];

        let task = create_test_task(
            &mut tx,
            TaskCreate::default(),
            None,
            None,
            base_time,
            base_time,
        )
        .await;

        let res = update_task_tags_inner(
            &mut tx,
            task.id,
            Uuid::nil(),
            tag_list.iter().map(|tag| tag.id).collect(),
        )
        .await;
        assert!(res.is_ok());

        let tags = res.unwrap();
        for tag in tags {
            assert!(matches!(tag, Tag { .. }))
        }
    }

    #[test]
    async fn is_idempotent() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = get_time();

        let tag_list = vec![
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await,
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await,
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await,
        ];

        let task = create_test_task(
            &mut tx,
            TaskCreate::default(),
            None,
            None,
            base_time,
            base_time,
        )
        .await;

        let res = update_task_tags_inner(
            &mut tx,
            task.id,
            Uuid::nil(),
            tag_list.iter().map(|tag| tag.id).collect(),
        )
        .await;
        assert!(res.is_ok());

        let tags = res.unwrap();
        assert_eq!(tags, tag_list);

        let res = update_task_tags_inner(
            &mut tx,
            task.id,
            Uuid::nil(),
            tag_list.iter().map(|tag| tag.id).collect(),
        )
        .await;
        assert!(res.is_ok());

        let tags = res.unwrap();
        assert_eq!(tags, tag_list);
    }

    #[test]
    async fn empty_list() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = get_time();

        // No existing tags
        let task = create_test_task(
            &mut tx,
            TaskCreate::default(),
            None,
            None,
            base_time,
            base_time,
        )
        .await;

        let res = update_task_tags_inner(&mut tx, task.id, Uuid::nil(), vec![]).await;
        assert!(res.is_ok());

        let tags = res.unwrap();
        assert!(tags.is_empty());

        let ret_task = get_task(&mut tx, task.id).await;
        assert!(ret_task.tags.is_empty());

        // With one existing tag
        let tag_list = [create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await];

        let task = create_test_task(
            &mut tx,
            TaskCreate {
                tags: tag_list.iter().map(|tag| tag.id).collect(),
                ..Default::default()
            },
            None,
            None,
            base_time,
            base_time,
        )
        .await;

        let res = update_task_tags_inner(&mut tx, task.id, Uuid::nil(), vec![]).await;
        assert!(res.is_ok());

        let tags = res.unwrap();
        assert!(tags.is_empty());

        let ret_task = get_task(&mut tx, task.id).await;
        assert!(ret_task.tags.is_empty());

        // With many existing tags
        let tag_list = [
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await,
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await,
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await,
        ];

        let task = create_test_task(
            &mut tx,
            TaskCreate {
                tags: tag_list.iter().map(|tag| tag.id).collect(),
                ..Default::default()
            },
            None,
            None,
            base_time,
            base_time,
        )
        .await;

        let res = update_task_tags_inner(&mut tx, task.id, Uuid::nil(), vec![]).await;
        assert!(res.is_ok());

        let tags = res.unwrap();
        assert!(tags.is_empty());

        let ret_task = get_task(&mut tx, task.id).await;
        assert!(ret_task.tags.is_empty());
    }

    #[test]
    async fn one_tag() {
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

        let mut tag_list =
            vec![create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await];
        let res = update_task_tags_inner(&mut tx, task.id, Uuid::nil(), vec![tag_list[0].id]).await;
        assert!(res.is_ok());

        let ret_task = get_task(&mut tx, task.id).await;
        assert_eq!(ret_task.tags.len(), 1);
        assert_eq!(ret_task.tags, tag_list[..1]);

        tag_list.push(create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await);
        let res = update_task_tags_inner(&mut tx, task.id, Uuid::nil(), vec![tag_list[1].id]).await;
        assert!(res.is_ok());

        let ret_task = get_task(&mut tx, task.id).await;
        assert_eq!(ret_task.tags.len(), 1);
        assert_eq!(ret_task.tags, tag_list[1..2]);

        tag_list.push(create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await);
        let res = update_task_tags_inner(&mut tx, task.id, Uuid::nil(), vec![tag_list[2].id]).await;
        assert!(res.is_ok());

        let ret_task = get_task(&mut tx, task.id).await;
        assert_eq!(ret_task.tags.len(), 1);
        assert_eq!(ret_task.tags, tag_list[2..3]);
    }

    #[test]
    async fn many_tags() {
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

        let mut tag_list = vec![
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await,
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await,
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await,
        ];
        let res = update_task_tags_inner(
            &mut tx,
            task.id,
            Uuid::nil(),
            tag_list[..3].iter().map(|tag| tag.id).collect(),
        )
        .await;
        assert!(res.is_ok());

        let ret_task = get_task(&mut tx, task.id).await;
        assert_eq!(ret_task.tags.len(), 3);
        assert_eq!(ret_task.tags, tag_list[0..3]);

        tag_list.append(&mut vec![
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await,
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await,
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await,
        ]);
        let res = update_task_tags_inner(
            &mut tx,
            task.id,
            Uuid::nil(),
            tag_list[3..6].iter().map(|tag| tag.id).collect(),
        )
        .await;
        assert!(res.is_ok());

        let ret_task = get_task(&mut tx, task.id).await;
        assert_eq!(ret_task.tags.len(), 3);
        assert_eq!(ret_task.tags, tag_list[3..6]);

        tag_list.append(&mut vec![
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await,
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await,
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await,
        ]);
        let res = update_task_tags_inner(
            &mut tx,
            task.id,
            Uuid::nil(),
            tag_list[6..9].iter().map(|tag| tag.id).collect(),
        )
        .await;
        assert!(res.is_ok());

        let ret_task = get_task(&mut tx, task.id).await;
        assert_eq!(ret_task.tags.len(), 3);
        assert_eq!(ret_task.tags, tag_list[6..9]);

        let res = update_task_tags_inner(
            &mut tx,
            task.id,
            Uuid::nil(),
            tag_list[3..6].iter().map(|tag| tag.id).collect(),
        )
        .await;
        assert!(res.is_ok());

        let ret_task = get_task(&mut tx, task.id).await;
        assert_eq!(ret_task.tags.len(), 3);
        assert_eq!(ret_task.tags, tag_list[3..6]);
    }

    #[test]
    async fn nonexistent_tag_one() {
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

        let res = update_task_tags_inner(&mut tx, task.id, Uuid::nil(), vec![Uuid::new_v4()]).await;
        assert!(res.is_err());
        if let Err(err) = res {
            assert!(matches!(
                err,
                Error::Application(ApplicationError::TagNotFound)
            ))
        }
    }

    #[test]
    async fn nonexistent_tag_within() {
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

        let tag_ids = vec![
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time)
                .await
                .id,
            Uuid::new_v4(),
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time)
                .await
                .id,
            Uuid::new_v4(),
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time)
                .await
                .id,
            Uuid::new_v4(),
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time)
                .await
                .id,
            Uuid::new_v4(),
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time)
                .await
                .id,
            Uuid::new_v4(),
        ];

        let res = update_task_tags_inner(&mut tx, task.id, Uuid::nil(), tag_ids).await;
        assert!(res.is_err());
        if let Err(err) = res {
            assert!(matches!(
                err,
                Error::Application(ApplicationError::TagNotFound)
            ))
        }
    }

    #[test]
    async fn nonexistent_tag_all() {
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

        let tag_ids = vec![
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
        ];

        let res = update_task_tags_inner(&mut tx, task.id, Uuid::nil(), tag_ids).await;
        assert!(res.is_err());
        if let Err(err) = res {
            assert!(matches!(
                err,
                Error::Application(ApplicationError::TagNotFound)
            ))
        }
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

        let res = update_task_tags_inner(&mut tx, task.id, Uuid::nil(), vec![]).await;
        assert!(res.is_err());
        if let Err(err) = res {
            assert!(matches!(
                err,
                Error::Application(ApplicationError::TaskNotFound)
            ))
        }
    }

    #[test]
    async fn nonexistent_task() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let res = update_task_tags_inner(&mut tx, Uuid::new_v4(), Uuid::nil(), vec![]).await;
        assert!(res.is_err());
        if let Err(err) = res {
            assert!(matches!(
                err,
                Error::Application(ApplicationError::TaskNotFound)
            ))
        }
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
            Some(base_time),
            base_time,
            base_time,
        )
        .await;

        let res = update_task_tags_inner(&mut tx, task.id, Uuid::nil(), vec![]).await;
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
mod delete_tests {
    use tokio::test;
    use uuid::Uuid;

    use super::delete_task_tag_inner;
    use crate::db::test_utils::{create_test_tag, create_test_task, get_pool, get_task, get_time};
    use crate::db::{ApplicationError, Error};
    use crate::routes::models::{tag::Model as TagCreate, task::Model as TaskCreate};

    #[test]
    async fn base_delete() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = get_time();

        let tag_list = [
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await,
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await,
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await,
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await,
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await,
        ];

        let task = create_test_task(
            &mut tx,
            TaskCreate {
                tags: tag_list.iter().map(|tag| tag.id).collect(),
                ..Default::default()
            },
            None,
            None,
            base_time,
            base_time,
        )
        .await;
        assert_eq!(task.tags.len(), tag_list.len());

        let res = delete_task_tag_inner(&mut tx, task.id, Uuid::nil(), tag_list[2].id).await;
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), ());

        let ret_task = get_task(&mut tx, task.id).await;
        assert_eq!(ret_task.tags.len(), 4);
        assert!(!ret_task.tags.contains(&tag_list[2]))
    }

    #[test]
    async fn is_idempotent() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = get_time();

        let tag_list = [
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await,
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await,
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await,
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await,
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await,
        ];

        let task = create_test_task(
            &mut tx,
            TaskCreate {
                tags: tag_list.iter().map(|tag| tag.id).collect(),
                ..Default::default()
            },
            None,
            None,
            base_time,
            base_time,
        )
        .await;
        assert_eq!(task.tags.len(), tag_list.len());

        let res = delete_task_tag_inner(&mut tx, task.id, Uuid::nil(), tag_list[2].id).await;
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), ());

        let ret_task = get_task(&mut tx, task.id).await;
        assert_eq!(ret_task.tags.len(), 4);
        assert!(!ret_task.tags.contains(&tag_list[2]));

        let res = delete_task_tag_inner(&mut tx, task.id, Uuid::nil(), tag_list[2].id).await;
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), ());

        let ret_task = get_task(&mut tx, task.id).await;
        assert_eq!(ret_task.tags.len(), 4);
        assert!(!ret_task.tags.contains(&tag_list[2]));
    }

    #[test]
    async fn tag_not_on_task() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = get_time();

        let tag_list = [
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await,
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await,
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await,
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await,
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await,
        ];

        let task = create_test_task(
            &mut tx,
            TaskCreate {
                tags: vec![tag_list[0].id, tag_list[2].id, tag_list[4].id],
                ..Default::default()
            },
            None,
            None,
            base_time,
            base_time,
        )
        .await;
        assert_eq!(task.tags.len(), 3);

        let res = delete_task_tag_inner(&mut tx, task.id, Uuid::nil(), tag_list[1].id).await;
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), ());

        let ret_task = get_task(&mut tx, task.id).await;
        assert_eq!(ret_task.tags.len(), 3);

        let res = delete_task_tag_inner(&mut tx, task.id, Uuid::nil(), tag_list[3].id).await;
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), ());

        let ret_task = get_task(&mut tx, task.id).await;
        assert_eq!(ret_task.tags.len(), 3);

        let res = delete_task_tag_inner(&mut tx, task.id, Uuid::nil(), Uuid::new_v4()).await;
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), ());

        let ret_task = get_task(&mut tx, task.id).await;
        assert_eq!(ret_task.tags.len(), 3);
    }

    #[test]
    async fn nonexistent_tag() {
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

        let res = delete_task_tag_inner(&mut tx, task.id, Uuid::nil(), Uuid::new_v4()).await;
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), ());

        let ret_task = get_task(&mut tx, task.id).await;
        assert_eq!(ret_task.tags.len(), 0);
    }

    #[test]
    async fn deleted_task() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = get_time();

        let tag = create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await;

        let task = create_test_task(
            &mut tx,
            TaskCreate::default(),
            None,
            Some(base_time),
            base_time,
            base_time,
        )
        .await;

        let res = delete_task_tag_inner(&mut tx, task.id, Uuid::nil(), tag.id).await;
        assert!(res.is_err());
        if let Err(err) = res {
            assert!(matches!(
                err,
                Error::Application(ApplicationError::TaskNotFound)
            ))
        }
    }

    #[test]
    async fn nonexistent_task() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let res = delete_task_tag_inner(&mut tx, Uuid::nil(), Uuid::nil(), Uuid::new_v4()).await;
        assert!(res.is_err());
        if let Err(err) = res {
            assert!(matches!(
                err,
                Error::Application(ApplicationError::TaskNotFound)
            ))
        }
    }

    #[test]
    async fn as_nonexistent_user() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = get_time();

        let tag = create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await;

        let task = create_test_task(
            &mut tx,
            TaskCreate {
                tags: vec![tag.id],
                ..Default::default()
            },
            None,
            Some(base_time),
            base_time,
            base_time,
        )
        .await;

        let res = delete_task_tag_inner(&mut tx, task.id, Uuid::new_v4(), tag.id).await;
        assert!(res.is_err());
        if let Err(err) = res {
            assert!(matches!(
                err,
                Error::Application(ApplicationError::TaskNotFound)
            ))
        }
    }
}
