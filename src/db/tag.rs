use sqlx::PgPool;
use uuid::Uuid;

use super::Result;
use crate::{com::model::Tag, db::query_as_wrapper};

pub async fn query_tags(
    conn: &PgPool,
    user_id: String,
    limit: i64,
    offset: i64,
) -> Result<Vec<Tag>> {
    // TODO: later allow filtering of tags by name search or category
    Ok(query_as_wrapper::<Tag>(
        "SELECT *
            FROM app.tags
            WHERE created_by = $1
            LIMIT $2 OFFSET $3",
    )
    .bind(user_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(conn)
    .await?)
}

pub async fn insert_tag(conn: &PgPool, user_id: String, tag: Tag) -> Result<Uuid> {
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

pub async fn select_tag(conn: &PgPool, tag_id: Uuid, user_id: String) -> Result<Option<Tag>> {
    let tag_opt = query_as_wrapper::<Tag>(
        "SELECT *
        FROM app.tags
        WHERE id = $1 AND created_by = $2",
    )
    .bind(tag_id)
    .bind(user_id)
    .fetch_optional(conn)
    .await?;

    match tag_opt {
        None => Ok(None),
        Some(tag) => Ok(Some(tag)),
    }
}

pub async fn update_tag(
    conn: &PgPool,
    tag_id: Uuid,
    user_id: String,
    tag: Tag,
) -> Result<Option<Tag>> {
    let mut transaction = conn.begin().await?;

    if sqlx::query(
        "UPDATE app.tags SET
        (updated_at, label, category) =
        (CURRENT_TIMESTAMP, $3, $4)
        WHERE id = $1 AND created_by = $2",
    )
    .bind(tag_id)
    .bind(&user_id)
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

pub async fn delete_tag(conn: &PgPool, tag_id: Uuid, user_id: String) -> Result<Option<()>> {
    let mut transaction = conn.begin().await?;

    if sqlx::query("DELETE FROM app.tags WHERE id = $1 AND created_by = $2")
        .bind(tag_id)
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
