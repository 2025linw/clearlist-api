use sqlx::{PgPool, Postgres};
use uuid::Uuid;

use super::Result;
use crate::{com::model::Tag, db::query_as_wrapper};

pub async fn query_tags(conn: &PgPool, limit: i64, offset: i64) -> Result<Vec<Tag>> {
    Ok(sqlx::query_as::<Postgres, Tag>(
        "SELECT id, label, category
            FROM app.tags
            LIMIT $1 OFFSET $2",
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(conn)
    .await?)
}

pub async fn insert_tag(conn: &PgPool, tag: Tag) -> Result<Uuid> {
    let mut transaction = conn.begin().await?;

    let tag_id = sqlx::query_scalar(
        "INSERT INTO app.tags (label, category)
        VALUES ($1, $2)
        RETURNING id",
    )
    .bind(tag.label)
    .bind(tag.category)
    .fetch_one(&mut *transaction)
    .await?;

    transaction.commit().await?;

    Ok(tag_id)
}

pub async fn select_tag(conn: &PgPool, tag_id: Uuid) -> Result<Option<Tag>> {
    let tag_opt = query_as_wrapper::<Tag>(
        "SELECT id, label, category
        FROM app.tags
        WHERE id = $1",
    )
    .bind(tag_id)
    .fetch_optional(conn)
    .await?;

    match tag_opt {
        None => Ok(None),
        Some(tag) => Ok(Some(tag)),
    }
}

pub async fn update_tag(conn: &PgPool, tag_id: Uuid, tag: Tag) -> Result<Option<Tag>> {
    let mut transaction = conn.begin().await?;

    if sqlx::query(
        "UPDATE app.tags SET
        (label, category) =
        ($2, $3)
        WHERE id = $1",
    )
    .bind(tag_id)
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

    select_tag(conn, tag_id).await
}

pub async fn delete_tag(conn: &PgPool, tag_id: Uuid) -> Result<Option<()>> {
    let mut transaction = conn.begin().await?;

    if sqlx::query("DELETE FROM app.tags WHERE id = $1")
        .bind(tag_id)
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
