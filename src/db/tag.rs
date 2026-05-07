//! # Database Tag Module
//!
//! This module contains collection of database functions for tags

use sqlx::{PgConnection, PgPool, QueryBuilder};
use uuid::Uuid;

use super::{ApplicationError, Error, Result, filters::TagSort, query_as_wrapper};
use crate::{
    com::constants::{DEFAULT_LIMIT, MAX_LIMIT},
    models::Tag,
    routes::models::{SortOrder, tag::Model as TagCreate},
};

/// Options for query tags in database
///
/// This contains filters for querying tags:
///
/// * `limit`: limits number of tags to return (default: 50)
/// * `offset`: number of tags to skip (default: 0)
/// * `sort_order`: order to return tags (default: recently updated first (updated decreasing))
pub struct TagQueryOptions {
    pub sort_order: TagSort,

    pub limit: i64,
    pub offset: i64,
}

impl Default for TagQueryOptions {
    fn default() -> Self {
        Self {
            sort_order: Default::default(),
            limit: DEFAULT_LIMIT,
            offset: 0,
        }
    }
}

/// Query database for tags
///
/// # Arguments
///
/// * `pool`: Database connection pool
/// * `user_id`: User ID to query tags for
/// * `opts`: Query filter
///
/// # Returns
///
/// List of tags
pub async fn query_tags(pool: PgPool, user_id: Uuid, opts: TagQueryOptions) -> Result<Vec<Tag>> {
    let mut conn = pool.acquire().await?;
    let tags = query_tags_inner(&mut conn, user_id, opts).await?;
    conn.close().await?;

    Ok(tags)
}

/// Internal function for `query_tags`
///
/// Only used internally
async fn query_tags_inner(
    conn: &mut PgConnection,
    user_id: Uuid,
    opts: TagQueryOptions,
) -> Result<Vec<Tag>> {
    let mut builder = QueryBuilder::new("SELECT * FROM app.tags WHERE created_by = ");
    builder.push_bind(user_id);
    match opts.sort_order {
        TagSort::Created(SortOrder::Ascending) => builder.push(" ORDER BY created_at ASC, id ASC"),
        TagSort::Created(SortOrder::Descending) => {
            builder.push(" ORDER BY created_at DESC, id ASC")
        }
        TagSort::Updated(SortOrder::Ascending) => builder.push(" ORDER BY updated_at ASC, id ASC"),
        TagSort::Updated(SortOrder::Descending) => {
            builder.push(" ORDER BY updated_at DESC, id ASC")
        }
        TagSort::Label(SortOrder::Ascending) => {
            builder.push(" ORDER BY LOWER(label) ASC, updated_at DESC, id ASC")
        }
        TagSort::Label(SortOrder::Descending) => {
            builder.push(" ORDER BY LOWER(label) DESC, updated_at DESC, id ASC")
        }
    };
    builder.push(" LIMIT ");
    builder.push_bind(opts.limit.clamp(1, MAX_LIMIT));
    builder.push(" OFFSET ");
    builder.push_bind(opts.offset.max(0));

    let query = builder.build_query_as::<Tag>();

    Ok(query.fetch_all(conn.as_mut()).await?)
}

/// Select tag from database
///
/// # Arguments
///
/// * `pool`: Database connection pool
/// * `tag_id`: ID of tag being retrieved
/// * `user_id`: User ID of tag owner
///
/// # Returns
///
/// Tag wrapped in `Some`, if tag exists
///
/// `None`, if it does not exist
pub async fn select_tag(pool: PgPool, tag_id: Uuid, user_id: Uuid) -> Result<Option<Tag>> {
    let mut conn = pool.acquire().await?;
    let tag_opt = select_tag_inner(&mut conn, tag_id, user_id).await?;
    conn.close().await?;

    Ok(tag_opt)
}

/// Internal function for `select_tag`
///
/// Only used internally
async fn select_tag_inner(
    conn: &mut PgConnection,
    tag_id: Uuid,
    user_id: Uuid,
) -> Result<Option<Tag>> {
    let tag_opt = query_as_wrapper::<Tag>(
        "SELECT *
        FROM app.tags
        WHERE id = $1 AND created_by = $2",
    )
    .bind(tag_id)
    .bind(user_id)
    .fetch_optional(conn.as_mut())
    .await?;

    Ok(tag_opt)
}

/// Inserts tag into database
///
/// # Arguments
///
/// * `pool`: Database connection pool
/// * `user_id`: User ID of tag owner
/// * `tag`: Tag being inserted
///
/// # Returns
///
/// Created tag
pub async fn insert_tag(pool: PgPool, user_id: Uuid, insert_tag: TagCreate) -> Result<Tag> {
    let mut tx = pool.begin().await?;
    let tag = insert_tag_inner(&mut tx, user_id, insert_tag).await?;
    tx.commit().await?;

    Ok(tag)
}

/// Internal function for `insert_tag`
///
/// Only used internally
async fn insert_tag_inner(
    conn: &mut PgConnection,
    user_id: Uuid,
    insert_tag: TagCreate,
) -> Result<Tag> {
    let res = query_as_wrapper::<Tag>(
        "INSERT INTO app.tags (label, category, created_by)
        VALUES ($1, NULLIF(trim($2), ''), $3)
        RETURNING *",
    )
    .bind(insert_tag.label)
    .bind(insert_tag.category)
    .bind(user_id)
    .fetch_one(conn.as_mut())
    .await;

    let tag_row = match res {
        Ok(row) => row,
        Err(err) => {
            if let Some(db_err) = err.as_database_error()
                && let Some("tags_created_by_fkey") = db_err.constraint()
            {
                return Err(Error::Application(ApplicationError::UserNotFound));
            }

            return Err(err.into());
        }
    };

    Ok(tag_row)
}

/// Update tag in database
///
/// # Arguments
///
/// * `pool`: Database connection pool
/// * `tag_id`: ID of tag being updated
/// * `user_id` User ID of tag owner
/// * `tag`: Updated tag
///
/// # Returns
///
/// Updated tag
pub async fn update_tag(
    pool: PgPool,
    tag_id: Uuid,
    user_id: Uuid,
    update_tag: TagCreate,
) -> Result<Tag> {
    let mut tx = pool.begin().await?;
    let tag = update_tag_inner(&mut tx, tag_id, user_id, update_tag).await?;
    tx.commit().await?;

    Ok(tag)
}

/// Internal function for `update_tag`
///
/// Only used internally
async fn update_tag_inner(
    conn: &mut PgConnection,
    tag_id: Uuid,
    user_id: Uuid,
    update_tag: TagCreate,
) -> Result<Tag> {
    let tag_opt = query_as_wrapper::<Tag>(
        "UPDATE app.tags
        SET (label, category)
        = ($3, NULLIF(trim($4), ''))
        WHERE id = $1 AND created_by = $2
        RETURNING *",
    )
    .bind(tag_id)
    .bind(user_id)
    .bind(update_tag.label)
    .bind(update_tag.category)
    .fetch_optional(conn.as_mut())
    .await?;

    if tag_opt.is_none() {
        return Err(Error::Application(ApplicationError::TagNotFound));
    }

    Ok(select_tag_inner(conn, tag_id, user_id)
        .await?
        .expect("tag was just updated"))
}

/// Deleted tag from database
///
/// # Arguments
///
/// * `pool`: Database connection pool
/// * `tag_id`: ID of tag being deleted
/// * `user_id`: User ID of tag owner
///
/// # Returns
///
/// Unit `()`
pub async fn delete_tag(pool: PgPool, tag_id: Uuid, user_id: Uuid) -> Result<()> {
    let mut tx = pool.begin().await?;
    delete_tag_inner(&mut tx, tag_id, user_id).await?;
    tx.commit().await?;

    Ok(())
}

/// Internal function for `delete_tag`
///
/// Only used internally
async fn delete_tag_inner(conn: &mut PgConnection, tag_id: Uuid, user_id: Uuid) -> Result<()> {
    sqlx::query("DELETE FROM app.tags WHERE id = $1 AND created_by = $2")
        .bind(tag_id)
        .bind(user_id)
        .execute(conn.as_mut())
        .await?;

    Ok(())
}

#[cfg(test)]
mod query {
    use std::{collections::HashSet, time::Duration};

    use chrono::Days;
    use tokio::test;
    use uuid::Uuid;

    use super::{TagQueryOptions, query_tags_inner};
    use crate::{
        com::constants::MAX_LIMIT,
        db::{
            filters::TagSort,
            test_utils::{create_test_tag, db_init},
        },
        routes::models::{SortOrder, tag::Model as TagCreate},
    };

    #[test]
    async fn default_query() {
        let (_, mut tx, base_time) = db_init().await;

        // create test data
        let _test_tags = [
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await,
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await,
            create_test_tag(
                &mut tx,
                TagCreate::default(),
                base_time,
                base_time + Duration::from_hours(1),
            )
            .await,
            create_test_tag(
                &mut tx,
                TagCreate::default(),
                base_time,
                base_time + Duration::from_hours(1),
            )
            .await,
            create_test_tag(
                &mut tx,
                TagCreate::default(),
                base_time,
                base_time + Duration::from_hours(2),
            )
            .await,
            create_test_tag(
                &mut tx,
                TagCreate::default(),
                base_time,
                base_time + Duration::from_hours(2),
            )
            .await,
        ];

        let opts = TagQueryOptions::default();

        let res = query_tags_inner(&mut tx, Uuid::nil(), opts).await;
        assert!(res.is_ok(), "query should always succeed");
        if let Ok(tags) = res {
            assert!(!tags.is_empty(), "must have data to test on");

            assert!(
                tags.is_sorted_by(|a, b| {
                    if a.updated_at > b.updated_at {
                        true
                    } else if a.updated_at < b.updated_at {
                        false
                    } else {
                        assert_ne!(a.id, b.id, "no duplicate tags should have been returned");

                        a.id < b.id
                    }
                }),
                "default query should have returned with updated_at descending, with id ascending as tiebreaker"
            )
        }
    }

    #[test]
    async fn sort_updated_ascending() {
        let (_, mut tx, base_time) = db_init().await;

        // create test data
        let _test_tags = [
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await,
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await,
            create_test_tag(
                &mut tx,
                TagCreate::default(),
                base_time,
                base_time + Duration::from_hours(1),
            )
            .await,
            create_test_tag(
                &mut tx,
                TagCreate::default(),
                base_time,
                base_time + Duration::from_hours(1),
            )
            .await,
            create_test_tag(
                &mut tx,
                TagCreate::default(),
                base_time,
                base_time + Duration::from_hours(2),
            )
            .await,
            create_test_tag(
                &mut tx,
                TagCreate::default(),
                base_time,
                base_time + Duration::from_hours(2),
            )
            .await,
        ];

        let opts = TagQueryOptions {
            sort_order: TagSort::Updated(SortOrder::Ascending),
            ..Default::default()
        };

        let res = query_tags_inner(&mut tx, Uuid::nil(), opts).await;
        assert!(res.is_ok(), "query should always succeed");
        if let Ok(tags) = res {
            assert!(!tags.is_empty(), "must have data to test on");

            assert!(
                tags.is_sorted_by(|a, b| {
                    if a.updated_at < b.updated_at {
                        true
                    } else if a.updated_at > b.updated_at {
                        false
                    } else {
                        assert_ne!(a.id, b.id, "no duplicate tags should have been returned");

                        a.id < b.id
                    }
                }),
                "expected sort by updated_at ascending, with id ascending as tiebreaker"
            )
        }
    }

    #[test]
    async fn sort_created_descending() {
        let (_, mut tx, base_time) = db_init().await;

        // create test data
        let _test_tags = [
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await,
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await,
            create_test_tag(
                &mut tx,
                TagCreate::default(),
                base_time + Duration::from_hours(1),
                base_time + Duration::from_hours(1),
            )
            .await,
            create_test_tag(
                &mut tx,
                TagCreate::default(),
                base_time + Duration::from_hours(1),
                base_time + Duration::from_hours(1),
            )
            .await,
            create_test_tag(
                &mut tx,
                TagCreate::default(),
                base_time + Duration::from_hours(2),
                base_time + Duration::from_hours(2),
            )
            .await,
            create_test_tag(
                &mut tx,
                TagCreate::default(),
                base_time + Duration::from_hours(2),
                base_time + Duration::from_hours(2),
            )
            .await,
        ];

        let opts = TagQueryOptions {
            sort_order: TagSort::Created(SortOrder::Descending),
            ..Default::default()
        };

        let res = query_tags_inner(&mut tx, Uuid::nil(), opts).await;
        assert!(res.is_ok(), "query should always succeed");
        if let Ok(tags) = res {
            assert!(!tags.is_empty(), "must have data to test on");

            assert!(
                tags.is_sorted_by(|a, b| {
                    if a.created_at > b.created_at {
                        true
                    } else if a.created_at < b.created_at {
                        false
                    } else {
                        assert_ne!(a.id, b.id, "no duplicate tag should have been returned");

                        a.id < b.id
                    }
                }),
                "expected sort by created_at descending, with id ascending as tiebreaker"
            )
        }
    }

    #[test]
    async fn sort_created_ascending() {
        let (_, mut tx, base_time) = db_init().await;

        // create test data
        let _test_tags = [
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await,
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await,
            create_test_tag(
                &mut tx,
                TagCreate::default(),
                base_time + Duration::from_hours(1),
                base_time + Duration::from_hours(1),
            )
            .await,
            create_test_tag(
                &mut tx,
                TagCreate::default(),
                base_time + Duration::from_hours(1),
                base_time + Duration::from_hours(1),
            )
            .await,
            create_test_tag(
                &mut tx,
                TagCreate::default(),
                base_time + Duration::from_hours(2),
                base_time + Duration::from_hours(2),
            )
            .await,
            create_test_tag(
                &mut tx,
                TagCreate::default(),
                base_time + Duration::from_hours(2),
                base_time + Duration::from_hours(2),
            )
            .await,
        ];

        let opts = TagQueryOptions {
            sort_order: TagSort::Created(SortOrder::Ascending),
            ..Default::default()
        };

        let res = query_tags_inner(&mut tx, Uuid::nil(), opts).await;
        assert!(res.is_ok(), "query should always succeed");
        if let Ok(tags) = res {
            assert!(!tags.is_empty(), "must have data to test on");

            assert!(
                tags.is_sorted_by(|a, b| {
                    if a.created_at < b.created_at {
                        true
                    } else if a.created_at > b.created_at {
                        false
                    } else {
                        assert_ne!(a.id, b.id, "no duplicate tag should have been returned");

                        a.id < b.id
                    }
                }),
                "expected sort by created_at ascending, with id ascending as tiebreaker"
            )
        }
    }

    #[test]
    async fn sort_label_ascending() {
        let (_, mut tx, base_time) = db_init().await;

        // create test data
        let _test_tags = [
            // Blank label
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await,
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await,
            create_test_tag(
                &mut tx,
                TagCreate::default(),
                base_time,
                base_time + Days::new(1),
            )
            .await,
            create_test_tag(
                &mut tx,
                TagCreate::default(),
                base_time,
                base_time + Days::new(1),
            )
            .await,
            // Tag A
            create_test_tag(
                &mut tx,
                TagCreate {
                    label: "Tag A".to_string(),
                    ..Default::default()
                },
                base_time,
                base_time,
            )
            .await,
            create_test_tag(
                &mut tx,
                TagCreate {
                    label: "Tag A".to_string(),
                    ..Default::default()
                },
                base_time,
                base_time,
            )
            .await,
            create_test_tag(
                &mut tx,
                TagCreate {
                    label: "Tag A".to_string(),
                    ..Default::default()
                },
                base_time,
                base_time + Days::new(1),
            )
            .await,
            create_test_tag(
                &mut tx,
                TagCreate {
                    label: "Tag A".to_string(),
                    ..Default::default()
                },
                base_time,
                base_time + Days::new(1),
            )
            .await,
            // Tag B
            create_test_tag(
                &mut tx,
                TagCreate {
                    label: "Tag B".to_string(),
                    ..Default::default()
                },
                base_time,
                base_time,
            )
            .await,
            create_test_tag(
                &mut tx,
                TagCreate {
                    label: "Tag B".to_string(),
                    ..Default::default()
                },
                base_time,
                base_time,
            )
            .await,
            create_test_tag(
                &mut tx,
                TagCreate {
                    label: "Tag B".to_string(),
                    ..Default::default()
                },
                base_time,
                base_time + Days::new(1),
            )
            .await,
            create_test_tag(
                &mut tx,
                TagCreate {
                    label: "Tag B".to_string(),
                    ..Default::default()
                },
                base_time,
                base_time + Days::new(1),
            )
            .await,
        ];

        let opts = TagQueryOptions {
            sort_order: TagSort::Label(SortOrder::Ascending),
            ..Default::default()
        };

        let res = query_tags_inner(&mut tx, Uuid::nil(), opts).await;
        assert!(res.is_ok(), "query should always succeed");
        if let Ok(tags) = res {
            assert!(!tags.is_empty(), "must have data to test on");

            assert!(
                tags.is_sorted_by(|a, b| {
                    if a.label < b.label {
                        true
                    } else if a.label > b.label {
                        false
                    } else if a.updated_at > b.updated_at {
                        true
                    } else if a.updated_at < b.updated_at {
                        false
                    } else {
                        assert_ne!(a.id, b.id, "no duplicate tag should have been returned");

                        a.id < b.id
                    }
                }),
                "expected sort by label ascending, with updated_at descending then id ascending as tiebreaker"
            )
        }
    }

    #[test]
    async fn sort_label_descending() {
        let (_, mut tx, base_time) = db_init().await;

        // create test data
        let _test_tags = [
            // Blank label
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await,
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await,
            create_test_tag(
                &mut tx,
                TagCreate::default(),
                base_time,
                base_time + Days::new(1),
            )
            .await,
            create_test_tag(
                &mut tx,
                TagCreate::default(),
                base_time,
                base_time + Days::new(1),
            )
            .await,
            // Tag A
            create_test_tag(
                &mut tx,
                TagCreate {
                    label: "Tag A".to_string(),
                    ..Default::default()
                },
                base_time,
                base_time,
            )
            .await,
            create_test_tag(
                &mut tx,
                TagCreate {
                    label: "Tag A".to_string(),
                    ..Default::default()
                },
                base_time,
                base_time,
            )
            .await,
            create_test_tag(
                &mut tx,
                TagCreate {
                    label: "Tag A".to_string(),
                    ..Default::default()
                },
                base_time,
                base_time + Days::new(1),
            )
            .await,
            create_test_tag(
                &mut tx,
                TagCreate {
                    label: "Tag A".to_string(),
                    ..Default::default()
                },
                base_time,
                base_time + Days::new(1),
            )
            .await,
            // Tag B
            create_test_tag(
                &mut tx,
                TagCreate {
                    label: "Tag B".to_string(),
                    ..Default::default()
                },
                base_time,
                base_time,
            )
            .await,
            create_test_tag(
                &mut tx,
                TagCreate {
                    label: "Tag B".to_string(),
                    ..Default::default()
                },
                base_time,
                base_time,
            )
            .await,
            create_test_tag(
                &mut tx,
                TagCreate {
                    label: "Tag B".to_string(),
                    ..Default::default()
                },
                base_time,
                base_time + Days::new(1),
            )
            .await,
            create_test_tag(
                &mut tx,
                TagCreate {
                    label: "Tag B".to_string(),
                    ..Default::default()
                },
                base_time,
                base_time + Days::new(1),
            )
            .await,
        ];

        let opts = TagQueryOptions {
            sort_order: TagSort::Label(SortOrder::Descending),
            ..Default::default()
        };

        let res = query_tags_inner(&mut tx, Uuid::nil(), opts).await;
        assert!(res.is_ok(), "query should always succeed");
        if let Ok(tags) = res {
            assert!(!tags.is_empty(), "must have data to test on");

            assert!(
                tags.is_sorted_by(|a, b| {
                    if a.label > b.label {
                        true
                    } else if a.label < b.label {
                        false
                    } else if a.updated_at > b.updated_at {
                        true
                    } else if a.updated_at < b.updated_at {
                        false
                    } else {
                        assert_ne!(a.id, b.id, "no duplicate tag should have been returned");

                        a.id < b.id
                    }
                }),
                "expected sort by label descending, with updated_at descending then id ascending as tiebreaker"
            )
        }
    }

    #[test]
    async fn limit() {
        let (_, mut tx, base_time) = db_init().await;

        // create test data
        for _ in 0..50 {
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await;
        }

        for limit in 1..=50 {
            let opts = TagQueryOptions {
                limit,
                ..Default::default()
            };

            let res = query_tags_inner(&mut tx, Uuid::nil(), opts).await;
            assert!(res.is_ok(), "query should always succeed");
            if let Ok(tags) = res {
                assert!(!tags.is_empty(), "must have data to test on");

                assert_eq!(
                    tags.len(),
                    limit as usize,
                    "limit does not match query filter value: {}",
                    limit
                );
            }
        }
    }

    #[test]
    async fn limit_0() {
        let (_, mut tx, base_time) = db_init().await;

        // create test data
        for _ in 0..50 {
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await;
        }

        let opts = TagQueryOptions {
            limit: 0,
            ..Default::default()
        };

        let res = query_tags_inner(&mut tx, Uuid::nil(), opts).await;
        assert!(res.is_ok(), "query should always succeed");
        if let Ok(tags) = res {
            assert!(!tags.is_empty(), "must have data to test on");

            assert_eq!(tags.len(), 1, "minimum limit should be clamped to 1");
        }
    }

    #[test]
    async fn limit_absurdly_large() {
        let (_, mut tx, base_time) = db_init().await;

        // create test data
        for _ in 0..250 {
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await;
        }

        let opts = TagQueryOptions {
            limit: i64::MAX,
            ..Default::default()
        };

        let res = query_tags_inner(&mut tx, Uuid::nil(), opts).await;
        assert!(res.is_ok(), "query should always succeed");
        if let Ok(tags) = res {
            assert!(!tags.is_empty(), "must have data to test on");

            assert_eq!(
                tags.len(),
                MAX_LIMIT as usize,
                "maximum limit should be clamped to MAX_LIMIT ({})",
                MAX_LIMIT
            );
        }
    }

    #[test]
    async fn limit_negative() {
        let (_, mut tx, base_time) = db_init().await;

        // create test data
        for _ in 0..10 {
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await;
        }

        let opts = TagQueryOptions {
            limit: -1,
            ..Default::default()
        };

        let res = query_tags_inner(&mut tx, Uuid::nil(), opts).await;
        assert!(res.is_ok(), "query should always succeed");
        if let Ok(tags) = res {
            assert!(!tags.is_empty(), "must have data to test on");

            assert_eq!(tags.len(), 1, "minimum limit should be clamped to 1");
        }

        let opts = TagQueryOptions {
            limit: -50,
            ..Default::default()
        };

        let res = query_tags_inner(&mut tx, Uuid::nil(), opts).await;
        assert!(res.is_ok(), "query should always succeed");
        if let Ok(tags) = res {
            assert!(!tags.is_empty(), "must have data to test on");

            assert_eq!(tags.len(), 1, "minimum limit should be clamped to 1");
        }

        let opts = TagQueryOptions {
            limit: i64::MIN,
            ..Default::default()
        };

        let res = query_tags_inner(&mut tx, Uuid::nil(), opts).await;
        assert!(res.is_ok(), "query should always succeed");
        if let Ok(tags) = res {
            assert!(!tags.is_empty(), "must have data to test on");

            assert_eq!(tags.len(), 1, "minimum limit should be clamped to 1");
        }
    }

    #[test]
    async fn limit_with_lots_of_data() {
        let (_, mut tx, base_time) = db_init().await;

        // create test data
        for _ in 0..1000 {
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await;
        }

        let opts = TagQueryOptions {
            limit: i64::MAX,
            ..Default::default()
        };

        let res = query_tags_inner(&mut tx, Uuid::nil(), opts).await;
        assert!(res.is_ok(), "query should always succeed");
        if let Ok(tags) = res {
            assert!(!tags.is_empty(), "must have data to test on");

            assert_eq!(
                tags.len(),
                MAX_LIMIT as usize,
                "maximum limit should be clamped to MAX_LIMIT ({})",
                MAX_LIMIT
            );
        }
    }

    #[test]
    async fn limit_with_paging_offset() {
        let (_, mut tx, base_time) = db_init().await;

        // create test data
        for _ in 0..254 {
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await;
        }

        let limit = 5;

        // keep paging until less than 'limit' tags are return
        let mut i = 0;
        let mut seen = HashSet::new();
        loop {
            let opts = TagQueryOptions {
                limit,
                offset: i * limit,
                ..Default::default()
            };

            let res = query_tags_inner(&mut tx, Uuid::nil(), opts).await;
            assert!(res.is_ok(), "query should always succeed");
            if let Ok(tags) = res {
                if i == 0 {
                    assert!(
                        !tags.is_empty(),
                        "first iteration: must have data to test on"
                    );
                }

                assert!(
                    tags.len() <= limit as usize,
                    "no more than `limit` tags should have been returned: found {}",
                    tags.len()
                );
                for tag in &tags {
                    assert!(seen.insert(tag.id), "duplicate tag encountered");
                }
                seen.extend(tags.iter().map(|t| t.id));

                i += 1;

                if tags.len() < limit as usize {
                    break;
                }
            }
        }

        // perform one more query to ensure that the end has been reached
        let opts = TagQueryOptions {
            limit,
            offset: i * limit,
            ..Default::default()
        };

        let res = query_tags_inner(&mut tx, Uuid::nil(), opts).await;
        assert!(res.is_ok(), "query should always succeed");
        if let Ok(tags) = res {
            assert!(tags.is_empty(), "should be past the end of the list");
        }
    }

    #[test]
    async fn offset_absurdly_large() {
        let (_, mut tx, base_time) = db_init().await;

        // create test data
        for _ in 0..250 {
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await;
        }

        let opts = TagQueryOptions {
            offset: i64::MAX,
            ..Default::default()
        };

        let res = query_tags_inner(&mut tx, Uuid::nil(), opts).await;
        assert!(res.is_ok(), "query should always succeed");
    }

    #[test]
    async fn offset_negative() {
        let (_, mut tx, base_time) = db_init().await;

        // create test data
        for _ in 0..250 {
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await;
        }

        let opts = TagQueryOptions {
            offset: -1,
            ..Default::default()
        };

        let res = query_tags_inner(&mut tx, Uuid::nil(), opts).await;
        assert!(res.is_ok(), "query should always succeed");
        let tags_1 = res.unwrap();
        assert!(!tags_1.is_empty(), "must have data to test on");

        let opts = TagQueryOptions {
            offset: -50,
            ..Default::default()
        };

        let res = query_tags_inner(&mut tx, Uuid::nil(), opts).await;
        assert!(res.is_ok(), "query should always succeed");
        let tags_2 = res.unwrap();
        assert_eq!(
            tags_1, tags_2,
            "should match as negative offsets default to offset of 0"
        );

        let opts = TagQueryOptions {
            offset: i64::MIN,
            ..Default::default()
        };

        let res = query_tags_inner(&mut tx, Uuid::nil(), opts).await;
        assert!(res.is_ok(), "query should always succeed");
        let tags_3 = res.unwrap();
        assert_eq!(
            tags_2, tags_3,
            "should match as negative offsets default to offset of 0"
        )
    }

    #[test]
    async fn offset_without_limit() {
        let (_, mut tx, base_time) = db_init().await;

        // create test data
        for _ in 0..250 {
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await;
        }

        let opts = TagQueryOptions {
            offset: 20,
            ..Default::default()
        };

        let res = query_tags_inner(&mut tx, Uuid::nil(), opts).await;
        assert!(res.is_ok(), "query should always succeed");
        if let Ok(tags) = res {
            assert!(!tags.is_empty(), "must have data to test on");

            let opts = TagQueryOptions {
                limit: 70,
                ..Default::default()
            };

            let ref_tags = query_tags_inner(&mut tx, Uuid::nil(), opts).await.unwrap();

            assert_eq!(
                tags,
                ref_tags[20..],
                "should return 50 tags offset by 20 tags"
            )
        }
    }

    #[test]
    async fn full_combination() {
        // Use all filters

        let (_, mut tx, _) = db_init().await;

        let opts = TagQueryOptions {
            sort_order: TagSort::Created(SortOrder::Ascending),
            limit: 5,
            offset: 10,
        };

        assert!(query_tags_inner(&mut tx, Uuid::nil(), opts).await.is_ok())
    }
}

#[cfg(test)]
mod select {
    use std::{collections::HashSet, time::Duration};

    use tokio::test;
    use uuid::Uuid;

    use super::select_tag_inner;
    use crate::{
        db::test_utils::{create_test_tag, db_init},
        routes::models::tag::Model as TagCreate,
    };

    #[test]
    async fn base_select() {
        let (_, mut tx, base_time) = db_init().await;

        let tag = create_test_tag(
            &mut tx,
            TagCreate::default(),
            base_time,
            base_time + Duration::from_hours(1),
        )
        .await;

        let res = select_tag_inner(&mut tx, tag.id, Uuid::nil()).await;
        assert!(res.is_ok());

        let tag_opt = res.unwrap();
        assert!(tag_opt.is_some());

        let ret_tag = tag_opt.unwrap();
        assert_eq!(ret_tag.id, tag.id);
        assert_eq!(ret_tag.label, "");
        assert!(ret_tag.category.is_none());
    }

    #[test]
    async fn many_various_tags() {
        let (_, mut tx, base_time) = db_init().await;

        let mut seen_ids = HashSet::new();
        for _ in 0..10 {
            let tag = create_test_tag(
                &mut tx,
                TagCreate::default(),
                base_time,
                base_time + Duration::from_hours(1),
            )
            .await;

            assert!(seen_ids.insert(tag.id), "duplicate tag encountered");
        }
        assert!(!seen_ids.is_empty());

        for id in seen_ids {
            let res = select_tag_inner(&mut tx, id, Uuid::nil()).await;
            assert!(res.is_ok());

            let tag_opt = res.unwrap();
            assert!(tag_opt.is_some());

            let tag = tag_opt.unwrap();
            assert_eq!(tag.id, id);
            assert_eq!(tag.label, "");
            assert!(tag.category.is_none());
        }
    }

    #[test]
    async fn nonexistent_tag() {
        let (_, mut tx, _) = db_init().await;

        let res = select_tag_inner(&mut tx, Uuid::new_v4(), Uuid::nil()).await;
        assert!(res.is_ok());

        let tag_opt = res.unwrap();
        assert!(tag_opt.is_none());
    }

    #[test]
    async fn as_nonexistent_user() {
        let (_, mut tx, base_time) = db_init().await;

        let tag = create_test_tag(
            &mut tx,
            TagCreate::default(),
            base_time,
            base_time + Duration::from_hours(1),
        )
        .await;

        let res = select_tag_inner(&mut tx, tag.id, Uuid::new_v4()).await;
        assert!(res.is_ok());

        let tag_opt = res.unwrap();
        assert!(tag_opt.is_none());
    }
}

#[cfg(test)]
mod insert {
    use tokio::test;
    use uuid::Uuid;

    use super::insert_tag_inner;
    use crate::{
        db::{ApplicationError, Error, test_utils::db_init},
        routes::models::tag::Model as TagCreate,
    };

    #[test]
    async fn base_insert() {
        let (_, mut tx, _) = db_init().await;

        let res = insert_tag_inner(&mut tx, Uuid::nil(), TagCreate::default()).await;
        assert!(res.is_ok());

        let tag = res.unwrap();
        assert_eq!(tag.label, "");
        assert!(tag.category.is_none());
    }

    #[test]
    async fn with_label() {
        let (_, mut tx, _) = db_init().await;

        let res = insert_tag_inner(
            &mut tx,
            Uuid::nil(),
            TagCreate {
                label: "This is a test label for with_label test".to_string(),
                ..Default::default()
            },
        )
        .await;
        assert!(res.is_ok());

        let tag = res.unwrap();
        assert_eq!(tag.label, "This is a test label for with_label test");
        assert!(tag.category.is_none());

        assert_eq!(
            tag.created_at, tag.updated_at,
            "created_at and updated_at should be the same when created"
        );
    }

    #[test]
    async fn with_category() {
        let (_, mut tx, _) = db_init().await;

        let res = insert_tag_inner(
            &mut tx,
            Uuid::nil(),
            TagCreate {
                category: Some("This is a test category for with_category test".to_string()),
                ..Default::default()
            },
        )
        .await;
        assert!(res.is_ok());

        let tag = res.unwrap();
        assert_eq!(tag.label, "");
        assert!(tag.category.is_some());
        assert_eq!(
            tag.category.unwrap(),
            "This is a test category for with_category test"
        );

        assert_eq!(
            tag.created_at, tag.updated_at,
            "created_at and updated_at should be the same when created"
        );
    }

    #[test]
    async fn as_nonexistent_user() {
        let (_, mut tx, _) = db_init().await;

        let res = insert_tag_inner(&mut tx, Uuid::new_v4(), TagCreate::default()).await;
        assert!(res.is_err());
        if let Err(err) = res {
            assert!(matches!(
                err,
                Error::Application(ApplicationError::UserNotFound)
            ));
        }
    }

    #[test]
    async fn combination_1() {
        let (_, mut tx, _) = db_init().await;

        let res = insert_tag_inner(
            &mut tx,
            Uuid::nil(),
            TagCreate {
                label: "Backlog".to_string(),
                category: Some("Workflow".to_string()),
            },
        )
        .await;
        assert!(res.is_ok());

        let tag = res.unwrap();
        assert_eq!(tag.label, "Backlog");
        assert!(tag.category.is_some());
        assert_eq!(tag.category.unwrap(), "Workflow");

        assert_eq!(
            tag.created_at, tag.updated_at,
            "created_at and updated_at should be the same when created"
        );
    }
}

#[cfg(test)]
mod update {
    use tokio::test;
    use uuid::Uuid;

    use super::update_tag_inner;
    use crate::{
        db::{
            ApplicationError, Error,
            test_utils::{create_test_tag, db_init},
        },
        models::Tag,
        routes::models::tag::Model as TagCreate,
    };

    fn verify_scope(after_tag: Tag, before_tag: Tag) {
        assert_eq!(after_tag.id, before_tag.id);
        assert_eq!(after_tag.created_at, before_tag.created_at);
        assert_eq!(after_tag.created_by, before_tag.created_by);
    }

    #[test]
    async fn is_idempotent() {
        let (_, mut tx, base_time) = db_init().await;

        let before_tag = create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await;

        let updated_tag: TagCreate = before_tag.clone().into();

        let res = update_tag_inner(&mut tx, before_tag.id, Uuid::nil(), updated_tag).await;
        assert!(res.is_ok());

        let after_tag = res.unwrap();
        assert_eq!(after_tag, before_tag);
    }

    #[test]
    async fn label_only() {
        let (_, mut tx, base_time) = db_init().await;

        let before_tag = create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await;

        let mut updated_tag: TagCreate = before_tag.clone().into();
        updated_tag.label = "New label".to_string();

        let res = update_tag_inner(&mut tx, before_tag.id, Uuid::nil(), updated_tag).await;
        assert!(res.is_ok());

        let after_tag = res.unwrap();
        assert_ne!(after_tag.updated_at, before_tag.updated_at);
        assert_ne!(after_tag.label, before_tag.label);
        assert_eq!(after_tag.label, "New label");

        assert_eq!(after_tag.category, after_tag.category);

        verify_scope(after_tag, before_tag);
    }

    #[test]
    async fn category_only() {
        let (_, mut tx, base_time) = db_init().await;

        let before_tag = create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await;

        let mut updated_tag: TagCreate = before_tag.clone().into();
        updated_tag.category = Some("Updated category".to_string());

        let res = update_tag_inner(&mut tx, before_tag.id, Uuid::nil(), updated_tag).await;
        assert!(res.is_ok());

        let after_tag = res.unwrap();
        assert_ne!(after_tag.updated_at, before_tag.updated_at);
        assert_ne!(after_tag.category, before_tag.category);
        assert!(after_tag.category.is_some());
        if let Some(ref category) = after_tag.category {
            assert_eq!(category, "Updated category");
        }

        assert_eq!(after_tag.label, before_tag.label);

        verify_scope(after_tag, before_tag);
    }

    #[test]
    async fn as_nonexistent_user() {
        let (_, mut tx, base_time) = db_init().await;

        let tag = create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await;

        let res = update_tag_inner(&mut tx, tag.id, Uuid::new_v4(), tag.clone().into()).await;
        assert!(res.is_err());
        if let Err(err) = res {
            assert!(matches!(
                err,
                Error::Application(ApplicationError::TagNotFound)
            ))
        }
    }

    #[test]
    async fn combination_1() {
        let (_, mut tx, base_time) = db_init().await;

        let before_tag = create_test_tag(
            &mut tx,
            TagCreate {
                label: "Inbox".to_string(),
                category: Some("Workflow".to_string()),
            },
            base_time,
            base_time,
        )
        .await;

        let mut updated_tag: TagCreate = before_tag.clone().into();
        updated_tag.label = "Backlog".to_string();

        let res = update_tag_inner(&mut tx, before_tag.id, Uuid::nil(), updated_tag).await;
        assert!(res.is_ok());

        let after_tag = res.unwrap();
        assert_ne!(after_tag.updated_at, before_tag.updated_at);
        assert_ne!(after_tag.label, before_tag.label);

        assert_eq!(after_tag.category, before_tag.category);
    }
}

#[cfg(test)]
mod delete {
    use tokio::test;
    use uuid::Uuid;

    use super::{delete_tag_inner, select_tag_inner};
    use crate::{
        db::test_utils::{create_test_tag, db_init},
        routes::models::tag::Model as TagCreate,
    };

    #[test]
    async fn base_delete() {
        let (_, mut tx, base_time) = db_init().await;

        let tag = create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await;

        let res = delete_tag_inner(&mut tx, tag.id, Uuid::nil()).await;
        assert!(res.is_ok());

        assert_eq!(res.unwrap(), (), "delete should return unit");

        let tag_opt = select_tag_inner(&mut tx, tag.created_by, Uuid::nil())
            .await
            .expect("select_tag_inner should succeed regardless of tag existence");
        assert!(tag_opt.is_none())
    }

    #[test]
    async fn is_idempotent() {
        let (_, mut tx, base_time) = db_init().await;

        let tag = create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await;

        let res = delete_tag_inner(&mut tx, tag.id, Uuid::nil()).await;
        assert!(res.is_ok());

        assert_eq!(res.unwrap(), (), "delete should return unit");

        let res = delete_tag_inner(&mut tx, tag.id, Uuid::nil()).await;
        assert!(res.is_ok());

        assert_eq!(res.unwrap(), (), "delete should return unit");
    }

    #[test]
    async fn nonexistent_tag() {
        let (_, mut tx, _) = db_init().await;

        let res = delete_tag_inner(&mut tx, Uuid::new_v4(), Uuid::nil()).await;
        assert!(res.is_ok());

        assert_eq!(res.unwrap(), (), "delete should return unit");
    }

    #[test]
    async fn as_nonexistent_user() {
        let (_, mut tx, base_time) = db_init().await;

        let tag = create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await;

        let res = delete_tag_inner(&mut tx, tag.id, Uuid::new_v4()).await;
        assert!(res.is_ok());
    }
}
