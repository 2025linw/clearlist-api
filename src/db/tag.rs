use sqlx::{PgPool, QueryBuilder};
use uuid::Uuid;

use super::Result;
use crate::{com::model::Tag, db::query_as_wrapper};

pub struct TagQueryOptions {
    pub user_id: Uuid,

    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub deleted: bool,
}

pub async fn query_tags(conn: PgPool, opts: TagQueryOptions) -> Result<Vec<Tag>> {
    // TODO: later allow filtering of tags by name search or category
    let mut builder = QueryBuilder::new("SELECT * FROM app.tags WHERE created_by = ");
    builder.push_bind(opts.user_id);
    if opts.deleted {
        builder.push(" AND deleted_at IS NOT NULL");
    } else {
        builder.push(" AND deleted_at IS NULL");
    }
    builder.push(" ORDER BY updated_at DESC");
if let Some(limit) = opts.limit {
    builder.push(format!(" LIMIT {}", limit));
}
    if let Some(offset) = opts.offset {
    builder.push(format!(" OFFSET {}", offset));
}

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

#[cfg(test)]
mod tests {
    use std::env;

    use std::sync::OnceLock;

    use tokio::test;

    use super::*;

    static DB_URL: OnceLock<String> = OnceLock::new();

    fn get_connect_url() -> &'static str {
        let conn = DB_URL.get_or_init(|| {
            dotenvy::from_filename(".env.testing").ok();

            let url: String = env::var("DATABASE_URL").unwrap();

            url
        });

        // run migrations

        conn
    }

    async fn setup() -> PgPool {
        dotenvy::from_filename(".env.testing").ok();

        PgPool::connect(get_connect_url()).await.unwrap()
    }

    #[test]
    async fn query_base() {
        let conn = setup().await;

        let tags = match query_tags(conn, Uuid::nil(), 100, 0, false).await {
            Ok(tags) => tags,
            Err(err) => {
                panic!("{}", err);
            }
        };

        assert_eq!(tags.len(), 6);
        for (n, tag) in (1..=6).zip(tags.iter()) {
            assert_eq!(tag.label, format!("Test Tag {}", n))
        }
    }

    #[test]
    async fn query_limit_3() {
        let conn = setup().await;

        let tags = match query_tags(conn, Uuid::nil(), 3, 0, false).await {
            Ok(tags) => tags,
            Err(err) => {
                panic!("{}", err);
            }
        };

        assert_eq!(tags.len(), 3);
        for (n, tag) in (1..=3).zip(tags.iter()) {
            assert_eq!(tag.label, format!("Test Tag {}", n));
        }
    }

    #[test]
    async fn query_limit_3_with_offset() {
        let conn = setup().await;

        for i in 0..2 {
            let i_min = i * 3;
            let i_max = i_min + 3;

            let tags = match query_tags(conn.clone(), Uuid::nil(), 3, i_min, false).await {
                Ok(tags) => tags,
                Err(err) => {
                    panic!("{}", err);
                }
            };

            assert_eq!(tags.len(), 3);
            for (i, tag) in (i_min..=i_max).zip(tags.iter()) {
                assert_eq!(tag.label, format!("Test Tag {}", i + 1));
            }
        }
    }

    #[test]
    async fn query_deleted() {
        let conn = setup().await;

        let tags = match query_tags(conn, Uuid::nil(), 100, 0, true).await {
            Ok(tags) => tags,
            Err(err) => {
                panic!("{}", err);
            }
        };

        assert_eq!(tags.len(), 3);
        for (n, tag) in (7..=9).zip(tags.iter()) {
            assert_eq!(tag.label, format!("Test Tag {}", n));
        }
    }

    #[test]
    async fn query_ensure_order_consistency() {
        let conn = setup().await;

        for _ in 0..10 {
            let tags = match query_tags(conn.clone(), Uuid::nil(), 100, 0, false).await {
                Ok(tags) => tags,
                Err(err) => {
                    panic!("{}", err);
                }
            };

            assert_eq!(tags.len(), 6);
            for (n, tag) in (1..=6).zip(tags.iter()) {
                assert_eq!(tag.label, format!("Test Tag {}", n))
            }
        }
    }
}
