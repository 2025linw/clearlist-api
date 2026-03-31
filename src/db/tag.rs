use sqlx::{PgPool, QueryBuilder};
use uuid::Uuid;

use crate::{
    com::model::Tag,
    db::{DEFAULT_LIMIT, MAX_LIMIT, query_as_wrapper},
};

use super::Result;

pub struct TagQueryOptions {
    pub user_id: Uuid,

    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub deleted: bool,
}

pub async fn query_tags(conn: PgPool, opts: TagQueryOptions) -> Result<Vec<Tag>> {
    let limit = opts.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
    let offset = opts.offset.unwrap_or(0).max(0);

    // TODO: later, allow filtering of tags by name search or category
    let mut builder = QueryBuilder::new("SELECT * FROM app.tags WHERE created_by = ");
    builder.push_bind(opts.user_id);
    if opts.deleted {
        builder.push(" AND deleted_at IS NOT NULL");
    } else {
        builder.push(" AND deleted_at IS NULL");
    }
    builder.push(" ORDER BY updated_at DESC");
    builder.push(" LIMIT ");
    builder.push_bind(limit);
    builder.push(" OFFSET ");
    builder.push_bind(offset);

    let query = builder.build_query_as::<Tag>();

    let tags = query.fetch_all(&conn).await?;

    Ok(tags)
}

pub async fn insert_tag(conn: PgPool, user_id: Uuid, tag: Tag) -> Result<Uuid> {
    let mut transaction = conn.begin().await?;

    let tag_id = sqlx::query_scalar(
        "INSERT INTO app.tags (label, category, created_by)
        VALUES ($1, $2, $3)
        RETURNING id",
    )
    .bind(tag.label)
    .bind(tag.category)
    .bind(user_id)
    .fetch_one(&mut *transaction)
    .await?;

    transaction.commit().await?;

    Ok(tag_id)
}

pub async fn select_tag(conn: PgPool, tag_id: Uuid, user_id: Uuid) -> Result<Option<Tag>> {
    let tag_opt = query_as_wrapper::<Tag>(
        "SELECT *
        FROM app.tags
        WHERE id = $1 AND created_by = $2 AND deleted_at IS NULL",
    )
    .bind(tag_id)
    .bind(user_id)
    .fetch_optional(&conn)
    .await?;

    match tag_opt {
        None => Ok(None),
        Some(tag) => Ok(Some(tag)),
    }
}

pub async fn update_tag(
    conn: PgPool,
    tag_id: Uuid,
    user_id: Uuid,
    tag: Tag,
) -> Result<Option<Tag>> {
    let mut transaction = conn.begin().await?;

    if sqlx::query(
        "UPDATE app.tags SET
        (updated_at, label, category) =
        (CURRENT_TIMESTAMP, $3, $4)
        WHERE id = $1 AND created_by = $2 AND deleted_at IS NULL",
    )
    .bind(tag_id)
    .bind(user_id)
    .bind(tag.label)
    .bind(tag.category)
    .execute(&mut *transaction)
    .await?
    .rows_affected()
        == 0
    {
        return Ok(None);
    }

    transaction.commit().await?;

    select_tag(conn, tag_id, user_id).await
}

pub async fn delete_tag(conn: PgPool, tag_id: Uuid, user_id: Uuid) -> Result<Option<()>> {
    let mut transaction = conn.begin().await?;

    if sqlx::query(
        "UPDATE app.tags SET
        (updated_at, deleted_at) =
        (CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
        WHERE id = $1 AND created_by = $2 AND deleted_at IS NULL",
    )
    .bind(tag_id)
    .bind(user_id)
    .execute(&mut *transaction)
    .await?
    .rows_affected()
        == 0
    {
        transaction.rollback().await?;
        return Ok(None);
    }

    transaction.commit().await?;

    Ok(Some(()))
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
