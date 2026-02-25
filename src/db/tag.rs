use deadpool_postgres::Object;
use tokio_postgres::Row;
use uuid::Uuid;

use crate::com::model::Tag;

use super::Result;

pub async fn query_tags(conn: &Object, limit: i64, offset: i64) -> Result<Vec<Tag>> {
    let rows: Vec<Row> = conn
        .query(
            "SELECT id, label, category
            FROM clear_list.tags
            LIMIT $1 OFFSET $2",
            &[&limit, &offset],
        )
        .await?;

    Ok(rows.into_iter().map(|row| row.into()).collect())
}

pub async fn insert_tag(conn: &mut Object, tag: Tag) -> Result<Uuid> {
    let transaction = conn.transaction().await?;

    let row: Row = transaction
        .query_one(
            "INSERT INTO clear_list.tags (label, category)
            VALUES ($1, $2)
            RETURNING id;",
            &[&tag.label, &tag.category],
        )
        .await?;

    transaction.commit().await?;

    Ok(row.get("id"))
}

pub async fn select_tag(conn: &Object, tag_id: Uuid) -> Result<Option<Tag>> {
    let row_opt = conn
        .query_opt(
            "SELECT id, label, category
            FROM clear_list.tags
            WHERE id = $1",
            &[&tag_id],
        )
        .await?;

    Ok(row_opt.map(|row| row.into()))
}

pub async fn update_tag(conn: &mut Object, tag_id: Uuid, tag: Tag) -> Result<Option<Tag>> {
    let transaction = conn.transaction().await?;

    if transaction
        .execute(
            "UPDATE clear_list.tags SET
            (label, category) =
            ($2, $3)
            WHERE id = $1;",
            &[&tag_id, &tag.label, &tag.category],
        )
        .await?
        != 1
    {
        return Ok(None);
    }

    transaction.commit().await?;

    select_tag(conn, tag_id).await
}

pub async fn delete_tag(conn: &mut Object, tag_id: Uuid) -> Result<Option<()>> {
    let transaction = conn.transaction().await?;

    if transaction
        .execute("DELETE FROM clear_list.tags WHERE id = $1;", &[&tag_id])
        .await?
        != 1
    {
        return Ok(None);
    }

    transaction.commit().await?;

    Ok(Some(()))
}
