use std::env;

use chrono::{DateTime, SubsecRound, Utc};
use sqlx::{Connection, PgConnection, PgPool, postgres::PgPoolOptions};
use tokio::sync::OnceCell;
use uuid::Uuid;

use crate::{
    db::query_as_wrapper,
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

/// Gets current time in Utc that matches the default PostgreSQL precision (6 decimals)
pub fn get_time() -> DateTime<Utc> {
    Utc::now().trunc_subsecs(PG_SUBSEC_PREC)
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
        "INSERT INTO app.tasks (title, notes, start_dt, has_time, deadline, created_by)
            VALUES ($1, $2, $3, $4, $5, $6) RETURNING id",
    )
    .bind(task.title.clone())
    .bind(task.notes.clone())
    .bind(task.start.as_ref().and_then(|s| match s.as_at() {
        Some(dt) => Some(dt),
        None => match s.as_on() {
            Some(d) => Some(d.and_hms_opt(0, 0, 0).unwrap().and_utc()),
            None => unreachable!(),
        },
    }))
    .bind(task.start.as_ref().is_some_and(|s| s.as_at().is_some()))
    .bind(task.deadline)
    .bind(Uuid::nil())
    .fetch_one(conn.as_mut())
    .await
    .unwrap();

    if !task.tags.is_empty() {
        sqlx::query("INSERT INTO app.task_tags (task_id, tag_id) SELECT $1, unnest_tag FROM UNNEST($2) AS unnest_tag")
        .bind(task_id)
        .bind(&task.tags)
        .execute(conn.as_mut())
        .await.unwrap();
    }

    sqlx::query(
        "UPDATE app.tasks SET (completed_at, deleted_at, created_at, updated_at)
        = ($3, $4, $5, $6) WHERE id = $1 AND created_by = $2",
    )
    .bind(task_id)
    .bind(Uuid::nil())
    .bind(completed_at)
    .bind(deleted_at)
    .bind(created_at)
    .bind(updated_at)
    .execute(conn.as_mut())
    .await
    .unwrap();

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
