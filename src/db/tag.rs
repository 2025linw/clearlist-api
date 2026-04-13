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
#[derive(Default)]
pub struct TagQueryOptions {
    pub limit: Option<i64>,
    pub offset: Option<i64>,

    pub sort_order: TagSort,
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
    let limit = opts.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
    let offset = opts.offset.unwrap_or(0).max(0);

    let mut builder = QueryBuilder::new("SELECT * FROM app.tags WHERE created_by = ");
    builder.push_bind(user_id);
    match opts.sort_order {
        TagSort::Updated(SortOrder::Descending) => builder.push(" ORDER BY updated_at DESC"),
        TagSort::Updated(SortOrder::Ascending) => builder.push(" ORDER BY updated_at ASC"),
        TagSort::Created(SortOrder::Descending) => builder.push(" ORDER BY created_at DESC"),
        TagSort::Created(SortOrder::Ascending) => builder.push(" ORDER BY created_at ASC"),
        TagSort::Label(SortOrder::Ascending) => builder.push(" ORDER BY label ASC"),
        TagSort::Label(SortOrder::Descending) => builder.push(" ORDER BY label DESC"),
    };
    builder.push(" LIMIT ");
    builder.push_bind(limit);
    builder.push(" OFFSET ");
    builder.push_bind(offset);

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
    let tag = query_as_wrapper::<Tag>(
        "INSERT INTO app.tags (label, category, created_by)
        VALUES ($1, $2, $3)
        RETURNING id",
    )
    .bind(insert_tag.label)
    .bind(insert_tag.category)
    .bind(user_id)
    .fetch_one(conn.as_mut())
    .await?;

    Ok(tag)
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
        "UPDATE app.tags SET
        (updated_at, label, category) =
        (CURRENT_TIMESTAMP, $3, $4)
        WHERE id = $1 AND created_by = $2 AND deleted_at IS NULL
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

// TODO: tag tests
// #[cfg(test)]
// mod query_tests {
//     use std::env;
//     use std::sync::OnceLock;

//     use tokio::test;

//     use super::*;

//     static DB_URL: OnceLock<String> = OnceLock::new();

//     fn get_connect_url() -> &'static str {
//         let conn = DB_URL.get_or_init(|| {
//             dotenvy::from_filename(".env.testing").ok();

//             let url: String = env::var("DATABASE_URL").unwrap();

//             url
//         });

//         // run migrations

//         conn
//     }

//     async fn setup() -> PgPool {
//         dotenvy::from_filename(".env.testing").ok();

//         PgPool::connect(get_connect_url()).await.unwrap()
//     }

//     // WARN: tests should be agnostic to any given state of the database
//     // This means that the test asserts should not rely on specific values if not testing for them or guaranteed in another way
//     // For example:
//     //     - Don't check list length if not limiting by LIMIT
//     //     - Don't rely on order of the name ('Test Tag 1', 'Test Tag 2', etc...)

//     // #[test]
//     // async fn query_base() {
//     //     let conn = setup().await;

//     //     let opts = TagQueryOptions {
//     //         user_id: Uuid::nil(),
//     //         limit: None,
//     //         offset: None,
//     //         deleted: false,
//     //     };

//     //     let tags = match query_tags(conn, opts).await {
//     //         Ok(tags) => tags,
//     //         Err(err) => {
//     //             panic!("{}", err);
//     //         }
//     //     };

//     //     // WARN: test failure here (see above)
//     //     assert_eq!(tags.len(), 6);
//     //     for (n, tag) in (1..=6).zip(tags.iter()) {
//     //         assert_eq!(tag.label, format!("Test Tag {}", n))
//     //     }
//     // }

//     // #[test]
//     // async fn query_limit_3() {
//     //     let conn = setup().await;

//     //     let opts = TagQueryOptions {
//     //         user_id: Uuid::nil(),
//     //         limit: Some(3),
//     //         offset: None,
//     //         deleted: false,
//     //     };

//     //     let tags = match query_tags(conn, opts).await {
//     //         Ok(tags) => tags,
//     //         Err(err) => {
//     //             panic!("{}", err);
//     //         }
//     //     };

//     //     // WARN: test failure here (see above)
//     //     assert_eq!(tags.len(), 3);
//     //     for (n, tag) in (1..=3).zip(tags.iter()) {
//     //         assert_eq!(tag.label, format!("Test Tag {}", n));
//     //     }
//     // }

//     // #[test]
//     // async fn query_limit_3_with_offset() {
//     //     let conn = setup().await;

//     //     for i in 0..2 {
//     //         let i_min = i * 3;
//     //         let i_max = i_min + 3;

//     //         let opts = TagQueryOptions {
//     //             user_id: Uuid::nil(),
//     //             limit: Some(3),
//     //             offset: Some(i * 3),
//     //             deleted: false,
//     //         };

//     //         let tags = match query_tags(conn.clone(), opts).await {
//     //             Ok(tags) => tags,
//     //             Err(err) => {
//     //                 panic!("{}", err);
//     //             }
//     //         };

//     //         assert_eq!(tags.len(), 3);
//     //         for (i, tag) in (i_min..=i_max).zip(tags.iter()) {
//     //             assert_eq!(tag.label, format!("Test Tag {}", i + 1));
//     //         }
//     //     }
//     // }

//     // #[test]
//     // async fn query_deleted() {
//     //     let conn = setup().await;

//     //     let opts = TagQueryOptions {
//     //         user_id: Uuid::nil(),
//     //         limit: None,
//     //         offset: None,
//     //         deleted: true,
//     //     };

//     //     let tags = match query_tags(conn, opts).await {
//     //         Ok(tags) => tags,
//     //         Err(err) => {
//     //             panic!("{}", err);
//     //         }
//     //     };

//     //     assert_eq!(tags.len(), 3);
//     //     for (n, tag) in (7..=9).zip(tags.iter()) {
//     //         assert_eq!(tag.label, format!("Test Tag {}", n));
//     //     }
//     // }

//     // #[test]
//     // async fn query_ensure_order_consistency() {
//     //     let conn = setup().await;

//     //     for _ in 0..10 {
//     //         let opts = TagQueryOptions {
//     //             user_id: Uuid::nil(),
//     //             limit: None,
//     //             offset: None,
//     //             deleted: false,
//     //         };

//     //         let tags = match query_tags(conn.clone(), opts).await {
//     //             Ok(tags) => tags,
//     //             Err(err) => {
//     //                 panic!("{}", err);
//     //             }
//     //         };

//     //         assert_eq!(tags.len(), 6);
//     //         for (n, tag) in (1..=6).zip(tags.iter()) {
//     //             assert_eq!(tag.label, format!("Test Tag {}", n))
//     //         }
//     //     }
//     // }
// }
