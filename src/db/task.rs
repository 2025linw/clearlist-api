use std::collections::{HashMap, hash_map::Entry};

use sqlx::{PgConnection, PgPool, QueryBuilder};
use uuid::Uuid;

use crate::{
    com::model::{
        Tag, Task, TaskIntermediate, TaskTagIntermediate,
        db::{DateFilter, SQLCmp, SortOrder},
    },
    db::{DEFAULT_LIMIT, MAX_LIMIT, query_as_wrapper},
};

use super::{Error, Result};

pub struct TaskQueryOptions {
    pub user_id: Uuid,

    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub completed: bool,
    pub deleted: bool,
    // TODO: This has weird behavior in different timezones... how to resolve
    pub start_filter: Option<DateFilter>,
    pub deadline_filter: Option<DateFilter>,
    pub sort_order: SortOrder,
}

pub async fn query_tasks(conn: PgPool, opts: TaskQueryOptions) -> Result<Vec<Task>> {
    let mut tx = conn.begin().await?;
    let tasks = query_tasks_inner(&mut tx, opts).await?;
    tx.commit().await?;

    Ok(tasks)
}

async fn query_tasks_inner(tx: &mut PgConnection, opts: TaskQueryOptions) -> Result<Vec<Task>> {
    let limit = opts.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
    let offset = opts.offset.unwrap_or(0).max(0);

    let mut builder = QueryBuilder::new("SELECT * FROM app.tasks WHERE created_by = ");
    builder.push_bind(opts.user_id);
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
            separated.push(format!("((start_on {} ", cmp));
            separated.push_bind_unseparated(date);
            separated.push_unseparated(format!(") OR (start_at::date {} ", cmp));
            separated.push_bind_unseparated(date);
            separated.push_unseparated(")");

            if matches!(cmp, SQLCmp::NotEqual) {
                separated.push_unseparated(" OR (start_on IS NULL AND start_at IS NULL)");
            }
            separated.push_unseparated(")");
        }
    }
    if let Some(deadline) = opts.deadline_filter {
        builder.push(" AND ");

        let mut separated = builder.separated(" AND ");
        for (cmp, date) in deadline.into_sql() {
            separated.push(format!("((deadline {} ", cmp));
            separated.push_bind_unseparated(date);
            separated.push_unseparated(")");

            if matches!(cmp, SQLCmp::NotEqual) {
                separated.push_unseparated(" OR (deadline IS NULL)");
            }
            separated.push_unseparated(")");
        }
    }
    builder.push(" GROUP BY id");
    match opts.sort_order {
        SortOrder::UpdatedDesc => builder.push(" ORDER BY updated_at DESC"),
        SortOrder::UpdatedAsc => builder.push(" ORDER BY updated_at ASC"),
        SortOrder::CreatedDesc => builder.push(" ORDER BY created_at DESC"),
        SortOrder::CreatedAsc => builder.push(" ORDER BY created_at ASC"),
    };
    builder.push(" LIMIT ");
    builder.push_bind(limit);
    builder.push(" OFFSET ");
    builder.push_bind(offset);

    let query = builder.build_query_as::<TaskIntermediate>();

    let mut tasks = query.fetch_all(tx.as_mut()).await?;

    // Get tags
    let task_ids: Vec<Uuid> = tasks.iter().map(|task| task.id).collect();
    let tags = query_as_wrapper::<TaskTagIntermediate>(
        "SELECT tt.task_id, tg.*
            FROM app.task_tags tt
            LEFT JOIN app.tags tg ON tt.tag_id = tg.id
            WHERE tt.task_id = ANY($1)",
    )
    .bind(task_ids)
    .fetch_all(tx.as_mut())
    .await?;

    let mut task_tag_map: HashMap<Uuid, Vec<Tag>> = HashMap::new();
    for TaskTagIntermediate { task_id, tag } in tags {
        if let Entry::Vacant(e) = task_tag_map.entry(task_id) {
            e.insert(vec![tag]);
        } else {
            task_tag_map.get_mut(&task_id).unwrap().push(tag);
        }
    }
    for task in tasks.iter_mut() {
        if task_tag_map.contains_key(&task.id) {
            task.tags = task_tag_map.remove(&task.id).unwrap();
        }
    }

    Ok(tasks.into_iter().map(|task| task.into()).collect())
}

pub async fn insert_task(conn: PgPool, user_id: Uuid, task: Task) -> Result<Uuid> {
    let mut tx = conn.begin().await?;
    let task_id = insert_task_inner(&mut tx, user_id, task).await?;
    tx.commit().await?;

    Ok(task_id)
}

async fn insert_task_inner(tx: &mut PgConnection, user_id: Uuid, task: Task) -> Result<Uuid> {
    let task_id = sqlx::query_scalar(
        "INSERT INTO app.tasks (title, notes, start_on, start_at, deadline, created_by)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING id",
    )
    .bind(task.title)
    .bind(task.notes)
    .bind(task.start.as_ref().map(|s| s.as_on()))
    .bind(task.start.as_ref().map(|s| s.as_at()))
    .bind(task.deadline)
    .bind(user_id)
    .fetch_one(tx.as_mut())
    .await?;

    if !task.tags.is_empty() {
        update_tag_helper(tx, task_id, task.tags).await?;
    }

    Ok(task_id)
}

pub async fn select_task(conn: PgPool, task_id: Uuid, user_id: Uuid) -> Result<Option<Task>> {
    let mut tx = conn.begin().await?;
    let task = select_task_inner(&mut tx, task_id, user_id).await?;
    tx.commit().await?;

    Ok(task)
}

async fn select_task_inner(
    tx: &mut PgConnection,
    task_id: Uuid,
    user_id: Uuid,
) -> Result<Option<Task>> {
    let task_opt = query_as_wrapper::<TaskIntermediate>(
        "SELECT *
        FROM app.tasks
        WHERE id = $1 AND created_by = $2 AND deleted_at IS NULL",
    )
    .bind(task_id)
    .bind(user_id)
    .fetch_optional(tx.as_mut())
    .await?;

    match task_opt {
        None => Ok(None),
        Some(mut task) => {
            let tags = query_as_wrapper::<Tag>(
                "SELECT tg.*
                FROM app.task_tags tt
                JOIN app.tags tg ON tt.tag_id = tg.id
                WHERE tt.task_id = $1 AND tg.deleted_at IS NULL",
            )
            .bind(task_id)
            .fetch_all(tx.as_mut())
            .await?;

            task.tags = tags;

            Ok(Some(task.into()))
        }
    }
}

pub async fn update_task(
    conn: PgPool,
    task_id: Uuid,
    user_id: Uuid,
    task: Task,
) -> Result<Option<Task>> {
    let mut tx = conn.begin().await?;
    let task_opt = update_task_inner(&mut tx, task_id, user_id, task).await?;
    tx.commit().await?;

    Ok(task_opt)
}

async fn update_task_inner(
    tx: &mut PgConnection,
    task_id: Uuid,
    user_id: Uuid,
    task: Task,
) -> Result<Option<Task>> {
    let task_opt = query_as_wrapper::<TaskIntermediate>(
        "UPDATE app.tasks SET
        (updated_at, title, notes, start_on, start_at, deadline) =
        (CURRENT_TIMESTAMP, $3, $4, $5, $6, $7)
        WHERE id = $1 AND created_by = $2 AND deleted_at IS NULL
        RETURNING *",
    )
    .bind(task_id)
    .bind(user_id)
    .bind(task.title)
    .bind(task.notes)
    .bind(task.start.clone().map(|s| s.as_on()))
    .bind(task.start.clone().map(|s| s.as_at()))
    .bind(task.deadline)
    .fetch_optional(tx.as_mut())
    .await?;

    let task = match task_opt {
        Some(task) => task,
        None => return Ok(None),
    };

    if !task.tags.is_empty() {
        update_tag_helper(tx, task_id, task.tags.clone()).await?;
    }

    Ok(Some(task.into()))
}

pub async fn delete_task(conn: PgPool, task_id: Uuid, user_id: Uuid) -> Result<Option<()>> {
    let mut tx = conn.begin().await?;
    let res = delete_task_inner(&mut tx, task_id, user_id).await?;
    tx.commit().await?;

    Ok(res)
}

async fn delete_task_inner(
    tx: &mut PgConnection,
    task_id: Uuid,
    user_id: Uuid,
) -> Result<Option<()>> {
    if sqlx::query(
        "UPDATE app.tasks SET
        (updated_at, deleted_at) =
        (CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
        WHERE id = $1 AND created_by = $2 AND deleted_at IS NULL",
    )
    .bind(task_id)
    .bind(user_id)
    .execute(tx.as_mut())
    .await?
    .rows_affected()
        == 0
    {
        return Ok(None);
    }

    Ok(Some(()))
}

pub async fn complete_task(
    conn: PgPool,
    task_id: Uuid,
    user_id: Uuid,
    completed: bool,
) -> Result<Option<()>> {
    let mut tx = conn.begin().await?;
    let res = complete_task_inner(&mut tx, task_id, user_id, completed).await?;
    tx.commit().await?;

    Ok(res)
}

pub async fn complete_task_inner(
    tx: &mut PgConnection,
    task_id: Uuid,
    user_id: Uuid,
    completed: bool,
) -> Result<Option<()>> {
    if completed {
        if sqlx::query(
            "UPDATE app.tasks SET
            (updated_at, completed_at) =
            (CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
            WHERE id = $1 AND created_by = $2 AND deleted_at IS NULL",
        )
        .bind(task_id)
        .bind(user_id)
        .execute(tx.as_mut())
        .await?
        .rows_affected()
            == 0
        {
            return Ok(None);
        }
    } else {
        if sqlx::query(
            "UPDATE app.tasks SET
            (updated_at, completed_at) =
            (CURRENT_TIMESTAMP, NULL)
            WHERE id = $1 AND created_by = $2 AND deleted_at IS NULL",
        )
        .bind(task_id)
        .bind(user_id)
        .execute(tx.as_mut())
        .await?
        .rows_affected()
            == 0
        {
            return Ok(None);
        }
    }

    Ok(Some(()))
}

pub async fn update_tag_helper(tx: &mut PgConnection, task_id: Uuid, tags: Vec<Tag>) -> Result<()> {
    let mut builder = QueryBuilder::new("INSERT INTO app.task_tags (task_id, tag_id) VALUES");

    let mut separated = builder.separated(", ");
    for tag in tags.iter() {
        separated.push(" (");
        separated.push_bind(task_id);
        separated.push(", ");
        separated.push_bind(tag.id);
        separated.push(")");
    }

    let num_rows = builder.build().execute(tx).await?.rows_affected() as usize;
    if num_rows != tags.len() {
        return Err(Error::Operation(format!(
            "expected: {} tags; got: {}",
            num_rows,
            tags.len()
        )));
    }

    Ok(())
}

pub mod tag {
    use sqlx::{PgPool, QueryBuilder};
    use uuid::Uuid;

    use crate::{com::model::Tag, db::query_as_wrapper};

    use super::Result;

    pub async fn query_task_tags(conn: PgPool, task_id: Uuid, user_id: Uuid) -> Result<Vec<Tag>> {
        Ok(query_as_wrapper::<Tag>(
            "SELECT *
                FROM app.tags tg
                JOIN app.task_tags tt ON tg.id = tt.tag_id
                JOIN app.tasks t ON tt.task_id = t.id AND t.deleted_at IS NULL
                WHERE tt.task_id = $1 AND t.created_by = $2 AND tg.deleted_at IS NULL",
        )
        .bind(task_id)
        .bind(user_id)
        .fetch_all(&conn)
        .await?)
    }

    pub async fn update_task_tags(
        conn: PgPool,
        task_id: Uuid,
        tag_ids: Vec<Uuid>,
    ) -> Result<Option<()>> {
        let mut builder =
            QueryBuilder::new("WITH deleted AS (DELETE FROM app.task_tags WHERE task_id = ");
        builder.push_bind(task_id);
        builder.push(" RETURNING task_id) INSERT INTO app.task_tags (task_id, tag_id) SELECT COALESCE(deleted.task_id, ");
        builder.push_bind(task_id);
        builder.push("), unnest_tag FROM deleted RIGHT JOIN UNNEST(");
        builder.push_bind(&tag_ids);
        builder.push(") AS unnest_tag ON TRUE");

        if builder.build().execute(&conn).await?.rows_affected() != tag_ids.len() as u64 {
            return Ok(None);
        }

        Ok(Some(()))
    }

    pub async fn add_task_tag(conn: PgPool, task_id: Uuid, tag_id: Uuid) -> Result<Option<()>> {
        sqlx::query("INSERT INTO app.task_tags (task_id, tag_id) VALUES ($1, $2)")
            .bind(task_id)
            .bind(tag_id)
            .execute(&conn)
            .await?;

        Ok(Some(()))
    }

    pub async fn delete_task_tag(conn: PgPool, task_id: Uuid, tag_id: Uuid) -> Result<Option<()>> {
        if sqlx::query("DELETE FROM app.task_tags WHERE task_id = $1 AND tag_id = $2")
            .bind(task_id)
            .bind(tag_id)
            .execute(&conn)
            .await?
            .rows_affected()
            == 0
        {
            return Ok(None);
        }

        Ok(Some(()))
    }
}

#[cfg(test)]
mod query_tests {
    use std::{collections::HashSet, env, path::Path, time::Duration};

    use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, Utc};
    use sqlx::{
        {Connection, PgConnection, PgPool},
        {migrate::Migrator, postgres::PgPoolOptions},
    };
    use tokio::{sync::OnceCell, test};
    use uuid::Uuid;

    use crate::{
        com::model::{
            Task,
            db::{DateBound, DateFilter, SortOrder},
            util::Start,
        },
        db::task::TaskQueryOptions,
    };

    use super::query_tasks_inner;

    const DATE_YEAR: i32 = 2027;
    const START_MONTH: u32 = 1;
    const DEADLINE_MONTH: u32 = 2;
    const DELETED_MONTH: u32 = 3;

    static POOL: OnceCell<PgPool> = OnceCell::const_new();
    async fn get_pool() -> &'static PgPool {
        POOL.get_or_init(|| async {
            dotenvy::from_filename("./.env.testing").ok();

            // migration
            let user = env::var("MIGRATION_USER").unwrap();
            let pass = env::var("MIGRATION_PASS").unwrap();
            let db = env::var("MIGRATION_DB").unwrap();
            let url = format!("postgresql://{}:{}@localhost/{}", user, pass, db);

            let mut conn = PgConnection::connect(&url).await.unwrap();

            let m = Migrator::new(Path::new("./migrations")).await.unwrap();
            m.run(&mut conn).await.unwrap();

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
            let pool = pool_opts.connect(&url).await.unwrap();

            pool
        })
        .await
    }

    fn create_default_opts() -> TaskQueryOptions {
        TaskQueryOptions {
            user_id: Uuid::nil(),
            limit: None,
            offset: None,
            completed: false,
            deleted: false,
            start_filter: None,
            deadline_filter: None,
            sort_order: SortOrder::default(),
        }
    }

    async fn insert_test_task(
        tx: &mut PgConnection,
        task: Task,
        completed_at: Option<DateTime<Utc>>,
        deleted_at: Option<DateTime<Utc>>,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) {
        sqlx::query(
            "INSERT INTO app.tasks (title, notes, start_on, start_at, deadline, created_by, completed_at, deleted_at, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
        )
        .bind(task.title)
        .bind(task.notes)
        .bind(task.start.as_ref().map(|s| s.as_on()))
        .bind(task.start.as_ref().map(|s| s.as_at()))
        .bind(task.deadline)
        .bind(Uuid::nil())
        .bind(completed_at)
        .bind(deleted_at)
        .bind(created_at)
        .bind(updated_at)
        .execute(tx)
        .await.ok();
    }

    async fn create_test_data(tx: &mut PgConnection) {
        // Create empty tasks
        let base_time = Utc::now();
        for i in 1..=10 {
            let title = format!("Test Task {}", i);

            let task = Task {
                title: title.clone(),
                notes: Some(format!("Notes for '{}'", title)),
                ..Default::default()
            };

            insert_test_task(
                tx,
                task,
                None,
                None,
                base_time + Duration::from_secs(i),
                base_time + Duration::from_hours(1) + Duration::from_secs(60 - i),
            )
            .await;
        }

        // Create start date tasks
        let base_time = Utc::now() + Duration::from_hours(1);
        for i in 1..=10 {
            let title = format!("Test Task SD{}", i);
            let date = NaiveDate::from_ymd_opt(DATE_YEAR, START_MONTH, i).unwrap();

            let task = Task {
                title: title.clone(),
                notes: Some(format!("Notes '{}'", title)),
                start: Some(Start::On(date)),
                ..Default::default()
            };

            insert_test_task(
                tx,
                task,
                None,
                None,
                base_time + Duration::from_secs(i as u64),
                base_time + Duration::from_hours(1) + Duration::from_secs(60 - i as u64),
            )
            .await;
        }

        // Create start datetime tasks
        let base_time = Utc::now() + Duration::from_hours(2);
        for i in 1..=10 {
            let title = format!("Test Task SDt{}", i);
            let date = NaiveDateTime::new(
                NaiveDate::from_ymd_opt(DATE_YEAR, START_MONTH, i).unwrap(),
                NaiveTime::from_hms_opt(12, 00, 00).unwrap(),
            );

            let task = Task {
                title: title.clone(),
                notes: Some(format!("Notes '{}'", title)),
                start: Some(Start::At(date.and_utc())),
                ..Default::default()
            };

            insert_test_task(
                tx,
                task,
                None,
                None,
                base_time + Duration::from_secs(i as u64),
                base_time + Duration::from_hours(1) + Duration::from_secs(60 - i as u64),
            )
            .await;
        }

        // Create deadline tasks
        let base_time = Utc::now() + Duration::from_hours(3);
        for i in 1..=10 {
            let title = format!("Test Task Dl{}", i);
            let date = NaiveDate::from_ymd_opt(DATE_YEAR, DEADLINE_MONTH, i).unwrap();

            let task = Task {
                title: title.clone(),
                notes: Some(format!("Notes '{}'", title)),
                deadline: Some(date),
                ..Default::default()
            };

            insert_test_task(
                tx,
                task,
                None,
                None,
                base_time + Duration::from_secs(i as u64),
                base_time + Duration::from_hours(1) + Duration::from_secs(60 - i as u64),
            )
            .await;
        }

        // Create completed tasks
        let base_time = Utc::now() + Duration::from_hours(4);
        for i in 1..=10 {
            let title = format!("Test Task Comp{}", i);
            let date = NaiveDate::from_ymd_opt(DATE_YEAR, DELETED_MONTH, i).unwrap();

            let task = Task {
                title: title.clone(),
                notes: Some(format!("Notes '{}'", title)),
                deadline: Some(date),
                ..Default::default()
            };

            insert_test_task(
                tx,
                task,
                Some(base_time + Duration::from_hours(2) + Duration::from_secs(i as u64)),
                None,
                base_time + Duration::from_secs(i as u64),
                base_time + Duration::from_hours(1) + Duration::from_secs(60 - i as u64),
            )
            .await;
        }

        // Create deleted tasks
        let base_time = Utc::now() + Duration::from_hours(5);
        for i in 1..=10 {
            let title = format!("Test Task Del{}", i);
            let date = NaiveDate::from_ymd_opt(DATE_YEAR, DELETED_MONTH, i).unwrap();

            let task = Task {
                title: title.clone(),
                notes: Some(format!("Notes '{}'", title)),
                deadline: Some(date),
                ..Default::default()
            };

            insert_test_task(
                tx,
                task,
                None,
                Some(base_time + Duration::from_hours(2) + Duration::from_secs(i as u64)),
                base_time + Duration::from_secs(i as u64),
                base_time + Duration::from_hours(1) + Duration::from_secs(60 - i as u64),
            )
            .await;
        }
    }

    #[test]
    async fn base_query() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        // create data
        create_test_data(&mut tx).await;

        let opts = create_default_opts();
        let tasks = query_tasks_inner(&mut tx, opts).await.unwrap();
        assert!(!tasks.is_empty(), "must have data to test on");

        println!("{:#?}", tasks);

        // default sort is updated_at descending
        assert!(tasks.is_sorted_by(|a, b| a.updated_at >= b.updated_at));

        for task in &tasks {
            // no deleted tasks
            assert!(task.deleted_at.is_none());
        }
    }

    #[test]
    async fn updated_by_ascending() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        // create data
        create_test_data(&mut tx).await;

        let mut opts = create_default_opts();
        opts.sort_order = SortOrder::UpdatedAsc;

        let tasks = query_tasks_inner(&mut tx, opts).await.unwrap();
        assert!(!tasks.is_empty(), "must have data to test on");

        // default sort is updated_at descending
        assert!(tasks.is_sorted_by(|a, b| a.updated_at <= b.updated_at));

        for task in &tasks {
            // no deleted tasks
            assert!(task.deleted_at.is_none());
        }
    }

    #[test]
    async fn created_by_descending() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        // create data
        create_test_data(&mut tx).await;

        let mut opts = create_default_opts();
        opts.sort_order = SortOrder::CreatedDesc;

        let tasks = query_tasks_inner(&mut tx, opts).await.unwrap();
        assert!(!tasks.is_empty(), "must have data to test on");

        // default sort is updated_at descending
        assert!(tasks.is_sorted_by(|a, b| a.created_at >= b.created_at));

        for task in &tasks {
            // no deleted tasks
            assert!(task.deleted_at.is_none());
        }
    }

    #[test]
    async fn created_by_ascending() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        // create data
        create_test_data(&mut tx).await;

        let mut opts = create_default_opts();
        opts.sort_order = SortOrder::CreatedAsc;

        let tasks = query_tasks_inner(&mut tx, opts).await.unwrap();
        assert!(!tasks.is_empty(), "must have data to test on");

        // default sort is updated_at descending
        assert!(tasks.is_sorted_by(|a, b| a.created_at <= b.created_at));

        for task in &tasks {
            // no deleted tasks
            assert!(task.deleted_at.is_none());
        }
    }

    #[test]
    async fn limit() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        // create data
        create_test_data(&mut tx).await;

        for i in 1..=10 {
            let mut opts = create_default_opts();
            opts.limit = Some(i);

            let tasks = query_tasks_inner(&mut tx, opts).await.unwrap();
            assert!(!tasks.is_empty(), "must have data to test on");

            for task in &tasks {
                // no deleted tasks
                assert!(task.deleted_at.is_none());
            }
        }
    }

    #[test]
    async fn limit_0() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        // create data
        create_test_data(&mut tx).await;

        let mut opts = create_default_opts();
        opts.limit = Some(0);

        query_tasks_inner(&mut tx, opts).await.unwrap();
    }

    #[test]
    async fn limit_absurdly_large() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        // create data
        create_test_data(&mut tx).await;

        let mut opts = create_default_opts();
        opts.limit = Some(i64::MAX);

        query_tasks_inner(&mut tx, opts).await.unwrap();
    }

    #[test]
    async fn limit_negative() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        // create data
        create_test_data(&mut tx).await;

        let mut opts = create_default_opts();
        opts.limit = Some(-1);

        query_tasks_inner(&mut tx, opts).await.unwrap();

        let mut opts = create_default_opts();
        opts.limit = Some(-50);

        query_tasks_inner(&mut tx, opts).await.unwrap();

        let mut opts = create_default_opts();
        opts.limit = Some(i64::MIN);

        query_tasks_inner(&mut tx, opts).await.unwrap();
    }

    #[test]
    async fn limit_with_paging_offset() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        // create data
        create_test_data(&mut tx).await;

        let limit = 5;

        // keep paging until less than 'limit' tasks are return
        let mut i = 0;
        let mut seen = HashSet::new();
        let mut first = true;
        loop {
            let mut opts = create_default_opts();
            opts.limit = Some(limit);
            opts.offset = Some(i * limit);

            let tasks = query_tasks_inner(&mut tx, opts).await.unwrap();
            if first {
                assert!(!tasks.is_empty(), "must have data to test on");
                first = false;
            }

            assert!(tasks.len() <= limit as usize);
            for task in &tasks {
                // no deleted tasks
                assert!(task.deleted_at.is_none());

                assert!(seen.insert(task.id), "duplicate task encountered");
            }
            seen.extend(tasks.iter().map(|t| t.id));

            i = i + 1;

            if tasks.len() < limit as usize {
                break;
            }
        }

        // perform one more query to ensure that the end has been reached
        let mut opts = create_default_opts();
        opts.limit = Some(limit);
        opts.offset = Some(i * limit);

        let tasks = query_tasks_inner(&mut tx, opts).await.unwrap();

        assert!(tasks.is_empty());
    }

    #[test]
    async fn offset_absurdly_large() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        // create data
        create_test_data(&mut tx).await;

        let mut opts = create_default_opts();
        opts.offset = Some(i64::MAX);

        query_tasks_inner(&mut tx, opts).await.unwrap();
    }

    #[test]
    async fn offset_negative() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        // create data
        create_test_data(&mut tx).await;

        let mut opts = create_default_opts();
        opts.offset = Some(-1);

        query_tasks_inner(&mut tx, opts).await.unwrap();

        let mut opts = create_default_opts();
        opts.offset = Some(-50);

        query_tasks_inner(&mut tx, opts).await.unwrap();

        let mut opts = create_default_opts();
        opts.offset = Some(i64::MIN);

        query_tasks_inner(&mut tx, opts).await.unwrap();
    }

    #[test]
    async fn offset_without_limits() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        // create data
        create_test_data(&mut tx).await;

        let mut opts = create_default_opts();
        opts.offset = Some(20);

        query_tasks_inner(&mut tx, opts).await.unwrap();
    }

    // TODO: add sort order to completed
    #[test]
    async fn filter_completed() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        // create data
        create_test_data(&mut tx).await;

        let mut opts = create_default_opts();
        opts.completed = true;

        let tasks = query_tasks_inner(&mut tx, opts).await.unwrap();
        assert!(!tasks.is_empty(), "must have data to test on");

        for task in &tasks {
            assert!(task.completed_at.is_some());
        }
    }

    // TODO: add sort order to deleted
    #[test]
    async fn filter_deleted() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        // create data
        create_test_data(&mut tx).await;

        let mut opts = create_default_opts();
        opts.deleted = true;

        let tasks = query_tasks_inner(&mut tx, opts).await.unwrap();
        assert!(!tasks.is_empty(), "must have data to test on");

        for task in &tasks {
            assert!(task.deleted_at.is_some());
        }
    }

    #[test]
    async fn filter_start_on() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        // create data
        create_test_data(&mut tx).await;

        let test_date = NaiveDate::from_ymd_opt(DATE_YEAR, START_MONTH, 5).unwrap();

        let mut opts = create_default_opts();
        opts.start_filter = Some(DateFilter::On(test_date));

        let tasks = query_tasks_inner(&mut tx, opts).await.unwrap();
        assert!(!tasks.is_empty(), "must have data to test on");

        for task in &tasks {
            // no deleted tasks
            assert!(task.deleted_at.is_none());

            assert!(matches!(task.start, Some(_)));

            match task.start.as_ref().unwrap() {
                Start::On(date) => assert_eq!(date, &test_date),
                Start::At(datetime) => assert_eq!(datetime.date_naive(), test_date),
            }
        }
    }

    #[test]
    async fn filter_start_not_on() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        // create data
        create_test_data(&mut tx).await;

        let test_date = NaiveDate::from_ymd_opt(DATE_YEAR, START_MONTH, 5).unwrap();

        let mut opts = create_default_opts();
        opts.start_filter = Some(DateFilter::NotOn(test_date));

        let tasks = query_tasks_inner(&mut tx, opts).await.unwrap();
        assert!(!tasks.is_empty(), "must have data to test on");

        for task in &tasks {
            // no deleted tasks
            assert!(task.deleted_at.is_none());

            if let Some(start) = &task.start {
                match start {
                    Start::On(date) => assert_ne!(date, &test_date),
                    Start::At(datetime) => assert_ne!(datetime.date_naive(), test_date),
                }
            }
        }
    }

    #[test]
    async fn filter_start_after_excl() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        // create data
        create_test_data(&mut tx).await;

        let test_date = NaiveDate::from_ymd_opt(DATE_YEAR, START_MONTH, 5).unwrap();

        let mut opts = create_default_opts();
        opts.start_filter = Some(DateFilter::StartRange(DateBound::Exclusive(test_date)));

        let tasks = query_tasks_inner(&mut tx, opts).await.unwrap();
        assert!(!tasks.is_empty(), "must have data to test on");

        for task in &tasks {
            // no deleted tasks
            assert!(task.deleted_at.is_none());

            assert!(matches!(task.start, Some(_)));

            match task.start.as_ref().unwrap() {
                Start::On(date) => assert!(date > &test_date),
                Start::At(datetime) => assert!(datetime.date_naive() > test_date),
            }
        }
    }

    #[test]
    async fn filter_start_after_incl() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        // create data
        create_test_data(&mut tx).await;

        let test_date = NaiveDate::from_ymd_opt(DATE_YEAR, START_MONTH, 5).unwrap();

        let mut opts = create_default_opts();
        opts.start_filter = Some(DateFilter::StartRange(DateBound::Inclusive(test_date)));

        let tasks = query_tasks_inner(&mut tx, opts).await.unwrap();
        assert!(!tasks.is_empty(), "must have data to test on");

        for task in &tasks {
            // no deleted tasks
            assert!(task.deleted_at.is_none());

            assert!(matches!(task.start, Some(_)));

            match task.start.as_ref().unwrap() {
                Start::On(date) => assert!(date >= &test_date),
                Start::At(datetime) => assert!(datetime.date_naive() >= test_date),
            }
        }
    }

    #[test]
    async fn filter_start_before_excl() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        // create data
        create_test_data(&mut tx).await;

        let test_date = NaiveDate::from_ymd_opt(DATE_YEAR, START_MONTH, 5).unwrap();

        let mut opts = create_default_opts();
        opts.start_filter = Some(DateFilter::EndRange(DateBound::Exclusive(test_date)));

        let tasks = query_tasks_inner(&mut tx, opts).await.unwrap();
        assert!(!tasks.is_empty(), "must have data to test on");

        for task in &tasks {
            // no deleted tasks
            assert!(task.deleted_at.is_none());

            assert!(matches!(task.start, Some(_)));

            match task.start.as_ref().unwrap() {
                Start::On(date) => assert!(date < &test_date),
                Start::At(datetime) => assert!(datetime.date_naive() < test_date),
            }
        }
    }

    #[test]
    async fn filter_start_before_incl() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        // create data
        create_test_data(&mut tx).await;

        let test_date = NaiveDate::from_ymd_opt(DATE_YEAR, START_MONTH, 5).unwrap();

        let mut opts = create_default_opts();
        opts.start_filter = Some(DateFilter::EndRange(DateBound::Inclusive(test_date)));

        let tasks = query_tasks_inner(&mut tx, opts).await.unwrap();
        assert!(!tasks.is_empty(), "must have data to test on");

        for task in &tasks {
            // no deleted tasks
            assert!(task.deleted_at.is_none());

            assert!(matches!(task.start, Some(_)));

            match task.start.as_ref().unwrap() {
                Start::On(date) => assert!(date <= &test_date),
                Start::At(datetime) => assert!(datetime.date_naive() <= test_date),
            }
        }
    }

    #[test]
    async fn filter_start_between_excl() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        // create data
        create_test_data(&mut tx).await;

        let test_date_min = NaiveDate::from_ymd_opt(DATE_YEAR, START_MONTH, 3).unwrap();
        let test_date_max = NaiveDate::from_ymd_opt(DATE_YEAR, START_MONTH, 6).unwrap();

        let mut opts = create_default_opts();
        opts.start_filter = Some(DateFilter::Range(
            DateBound::Exclusive(test_date_min),
            DateBound::Exclusive(test_date_max),
        ));

        let tasks = query_tasks_inner(&mut tx, opts).await.unwrap();
        assert!(!tasks.is_empty(), "must have data to test on");

        for task in &tasks {
            // no deleted tasks
            assert!(task.deleted_at.is_none());

            assert!(matches!(task.start, Some(_)));

            match task.start.as_ref().unwrap() {
                Start::On(date) => assert!(date > &test_date_min && date < &test_date_max),
                Start::At(datetime) => assert!(
                    datetime.date_naive() > test_date_min && datetime.date_naive() < test_date_max
                ),
            }
        }
    }

    #[test]
    async fn filter_start_between_incl() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        // create data
        create_test_data(&mut tx).await;

        let test_date_min = NaiveDate::from_ymd_opt(DATE_YEAR, START_MONTH, 3).unwrap();
        let test_date_max = NaiveDate::from_ymd_opt(DATE_YEAR, START_MONTH, 6).unwrap();

        let mut opts = create_default_opts();
        opts.start_filter = Some(DateFilter::Range(
            DateBound::Inclusive(test_date_min),
            DateBound::Inclusive(test_date_max),
        ));

        let tasks = query_tasks_inner(&mut tx, opts).await.unwrap();
        assert!(!tasks.is_empty(), "must have data to test on");

        for task in &tasks {
            // no deleted tasks
            assert!(task.deleted_at.is_none());

            assert!(matches!(task.start, Some(_)));

            match task.start.as_ref().unwrap() {
                Start::On(date) => assert!(date >= &test_date_min && date <= &test_date_max),
                Start::At(datetime) => assert!(
                    datetime.date_naive() >= test_date_min
                        && datetime.date_naive() <= test_date_max
                ),
            }
        }
    }

    #[test]
    async fn filter_start_between_combos() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        // create data
        create_test_data(&mut tx).await;

        let test_date_min = NaiveDate::from_ymd_opt(DATE_YEAR, START_MONTH, 3).unwrap();
        let test_date_max = NaiveDate::from_ymd_opt(DATE_YEAR, START_MONTH, 6).unwrap();

        // test exclusive max
        let mut opts = create_default_opts();
        opts.start_filter = Some(DateFilter::Range(
            DateBound::Inclusive(test_date_min),
            DateBound::Exclusive(test_date_max),
        ));

        let tasks = query_tasks_inner(&mut tx, opts).await.unwrap();
        assert!(!tasks.is_empty(), "must have data to test on");

        for task in &tasks {
            // no deleted tasks
            assert!(task.deleted_at.is_none());

            assert!(matches!(task.start, Some(_)));

            match task.start.as_ref().unwrap() {
                Start::On(date) => assert!(date >= &test_date_min && date < &test_date_max),
                Start::At(datetime) => assert!(
                    datetime.date_naive() >= test_date_min && datetime.date_naive() < test_date_max
                ),
            }
        }

        // text exclusive min
        let mut opts = create_default_opts();
        opts.start_filter = Some(DateFilter::Range(
            DateBound::Exclusive(test_date_min),
            DateBound::Inclusive(test_date_max),
        ));

        let tasks = query_tasks_inner(&mut tx, opts).await.unwrap();
        assert!(!tasks.is_empty(), "must have data to test on");

        for task in &tasks {
            // no deleted tasks
            assert!(task.deleted_at.is_none());

            assert!(matches!(task.start, Some(_)));

            match task.start.as_ref().unwrap() {
                Start::On(date) => assert!(date > &test_date_min && date <= &test_date_max),
                Start::At(datetime) => assert!(
                    datetime.date_naive() > test_date_min && datetime.date_naive() <= test_date_max
                ),
            }
        }
    }

    #[test]
    async fn filter_deadline_on() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        // create data
        create_test_data(&mut tx).await;

        let test_date = NaiveDate::from_ymd_opt(DATE_YEAR, DEADLINE_MONTH, 5).unwrap();

        let mut opts = create_default_opts();
        opts.deadline_filter = Some(DateFilter::On(test_date));

        let tasks = query_tasks_inner(&mut tx, opts).await.unwrap();
        assert!(!tasks.is_empty(), "must have data to test on");

        for task in &tasks {
            // no deleted tasks
            assert!(task.deleted_at.is_none());

            assert_eq!(task.deadline, Some(test_date));
        }
    }

    #[test]
    async fn filter_deadline_not_on() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        // create data
        create_test_data(&mut tx).await;

        let test_date = NaiveDate::from_ymd_opt(DATE_YEAR, DEADLINE_MONTH, 5).unwrap();

        let mut opts = create_default_opts();
        opts.deadline_filter = Some(DateFilter::NotOn(test_date));

        let tasks = query_tasks_inner(&mut tx, opts).await.unwrap();
        assert!(!tasks.is_empty(), "must have data to test on");

        for task in &tasks {
            // no deleted tasks
            assert!(task.deleted_at.is_none());

            match task.deadline {
                Some(date) => assert_ne!(date, test_date),
                None => (),
            }
        }
    }

    #[test]
    async fn filter_deadline_after_excl() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        // create data
        create_test_data(&mut tx).await;

        let test_date = NaiveDate::from_ymd_opt(DATE_YEAR, DEADLINE_MONTH, 5).unwrap();

        let mut opts = create_default_opts();
        opts.deadline_filter = Some(DateFilter::StartRange(DateBound::Exclusive(test_date)));

        let tasks = query_tasks_inner(&mut tx, opts).await.unwrap();
        assert!(!tasks.is_empty(), "must have data to test on");

        for task in &tasks {
            // no deleted tasks
            assert!(task.deleted_at.is_none());

            assert!(!matches!(task.deadline, None));

            assert!(task.deadline.unwrap() > test_date);
        }
    }

    #[test]
    async fn filter_deadline_after_incl() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        // create data
        create_test_data(&mut tx).await;

        let test_date = NaiveDate::from_ymd_opt(DATE_YEAR, DEADLINE_MONTH, 5).unwrap();

        let mut opts = create_default_opts();
        opts.deadline_filter = Some(DateFilter::StartRange(DateBound::Inclusive(test_date)));

        let tasks = query_tasks_inner(&mut tx, opts).await.unwrap();
        assert!(!tasks.is_empty(), "must have data to test on");

        for task in &tasks {
            // no deleted tasks
            assert!(task.deleted_at.is_none());

            assert!(!matches!(task.deadline, None));

            assert!(task.deadline.unwrap() >= test_date);
        }
    }

    #[test]
    async fn filter_deadline_before_excl() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        // create data
        create_test_data(&mut tx).await;

        let test_date = NaiveDate::from_ymd_opt(DATE_YEAR, DEADLINE_MONTH, 5).unwrap();

        let mut opts = create_default_opts();
        opts.deadline_filter = Some(DateFilter::EndRange(DateBound::Exclusive(test_date)));

        let tasks = query_tasks_inner(&mut tx, opts).await.unwrap();
        assert!(!tasks.is_empty(), "must have data to test on");

        for task in &tasks {
            // no deleted tasks
            assert!(task.deleted_at.is_none());

            assert!(!matches!(task.deadline, None));

            assert!(task.deadline.unwrap() < test_date);
        }
    }

    #[test]
    async fn filter_deadline_before_incl() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        // create data
        create_test_data(&mut tx).await;

        let test_date = NaiveDate::from_ymd_opt(DATE_YEAR, DEADLINE_MONTH, 5).unwrap();

        let mut opts = create_default_opts();
        opts.deadline_filter = Some(DateFilter::EndRange(DateBound::Inclusive(test_date)));

        let tasks = query_tasks_inner(&mut tx, opts).await.unwrap();
        assert!(!tasks.is_empty(), "must have data to test on");

        for task in &tasks {
            // no deleted tasks
            assert!(task.deleted_at.is_none());

            assert!(!matches!(task.deadline, None));

            assert!(task.deadline.unwrap() <= test_date);
        }
    }

    #[test]
    async fn filter_deadline_between_excl() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        // create data
        create_test_data(&mut tx).await;

        let test_date_min = NaiveDate::from_ymd_opt(DATE_YEAR, DEADLINE_MONTH, 3).unwrap();
        let test_date_max = NaiveDate::from_ymd_opt(DATE_YEAR, DEADLINE_MONTH, 6).unwrap();

        let mut opts = create_default_opts();
        opts.deadline_filter = Some(DateFilter::Range(
            DateBound::Exclusive(test_date_min),
            DateBound::Exclusive(test_date_max),
        ));

        let tasks = query_tasks_inner(&mut tx, opts).await.unwrap();
        assert!(!tasks.is_empty(), "must have data to test on");

        for task in &tasks {
            // no deleted tasks
            assert!(task.deleted_at.is_none());

            assert!(!matches!(task.deadline, None));

            assert!(
                task.deadline.unwrap() > test_date_min && task.deadline.unwrap() < test_date_max
            );
        }
    }

    #[test]
    async fn filter_deadline_between_incl() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        // create data
        create_test_data(&mut tx).await;

        let test_date_min = NaiveDate::from_ymd_opt(DATE_YEAR, DEADLINE_MONTH, 3).unwrap();
        let test_date_max = NaiveDate::from_ymd_opt(DATE_YEAR, DEADLINE_MONTH, 6).unwrap();

        let mut opts = create_default_opts();
        opts.deadline_filter = Some(DateFilter::Range(
            DateBound::Inclusive(test_date_min),
            DateBound::Inclusive(test_date_max),
        ));

        let tasks = query_tasks_inner(&mut tx, opts).await.unwrap();
        assert!(!tasks.is_empty(), "must have data to test on");

        for task in &tasks {
            // no deleted tasks
            assert!(task.deleted_at.is_none());

            assert!(!matches!(task.deadline, None));

            assert!(
                task.deadline.unwrap() >= test_date_min && task.deadline.unwrap() <= test_date_max
            );
        }
    }

    #[test]
    async fn as_nonexistent_user() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let mut opts = create_default_opts();
        opts.user_id = Uuid::new_v4();

        let tasks = query_tasks_inner(&mut tx, opts).await.unwrap();
        assert!(tasks.is_empty());
    }

    #[test]
    async fn full_combination() {
        // Use all filters

        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let date_mix = NaiveDate::from_ymd_opt(DATE_YEAR, START_MONTH, 1).unwrap();
        let date_max = NaiveDate::from_ymd_opt(DATE_YEAR, START_MONTH, 7).unwrap();

        let mut opts = create_default_opts();
        opts.limit = Some(5);
        opts.offset = Some(2);
        opts.start_filter = Some(DateFilter::Range(
            DateBound::Inclusive(date_mix),
            DateBound::Inclusive(date_max),
        ));
        opts.deadline_filter = Some(DateFilter::Range(
            DateBound::Inclusive(date_mix),
            DateBound::Inclusive(date_max),
        ));

        query_tasks_inner(&mut tx, opts).await.unwrap();
    }

    #[test]
    async fn full_combination_nondefault() {
        // Use all filters that are not the default option

        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let date_mix = NaiveDate::from_ymd_opt(DATE_YEAR, START_MONTH, 1).unwrap();
        let date_max = NaiveDate::from_ymd_opt(DATE_YEAR, START_MONTH, 7).unwrap();

        let mut opts = create_default_opts();
        opts.limit = Some(5);
        opts.offset = Some(2);
        opts.completed = true;
        opts.deleted = true;
        opts.start_filter = Some(DateFilter::Range(
            DateBound::Inclusive(date_mix),
            DateBound::Inclusive(date_max),
        ));
        opts.deadline_filter = Some(DateFilter::Range(
            DateBound::Inclusive(date_mix),
            DateBound::Inclusive(date_max),
        ));
        opts.sort_order = SortOrder::CreatedAsc;

        query_tasks_inner(&mut tx, opts).await.unwrap();
    }
}

#[cfg(test)]
mod create_tests {
    use std::{env, path::Path};

    use chrono::{Duration, Local, NaiveDate, NaiveDateTime, NaiveTime, SubsecRound, Utc};
    use sqlx::{Connection, PgConnection, PgPool, migrate::Migrator, postgres::PgPoolOptions};
    use tokio::{sync::OnceCell, test};
    use uuid::Uuid;

    use crate::com::model::{Task, util::Start};

    use super::{insert_task_inner, select_task_inner};

    const PG_SUBSEC_PREC: u16 = 6;

    static POOL: OnceCell<PgPool> = OnceCell::const_new();
    async fn get_pool() -> &'static PgPool {
        POOL.get_or_init(|| async {
            dotenvy::from_filename("./.env.testing").ok();

            // migration
            let user = env::var("MIGRATION_USER").unwrap();
            let pass = env::var("MIGRATION_PASS").unwrap();
            let db = env::var("MIGRATION_DB").unwrap();
            let url = format!("postgresql://{}:{}@localhost/{}", user, pass, db);

            let mut conn = PgConnection::connect(&url).await.unwrap();

            let m = Migrator::new(Path::new("./migrations")).await.unwrap();
            m.run(&mut conn).await.unwrap();

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
            let pool = pool_opts.connect(&url).await.unwrap();

            pool
        })
        .await
    }

    #[test]
    async fn base_insert() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let task = Task::default();

        let task_id = insert_task_inner(&mut tx, Uuid::nil(), task).await.unwrap();

        let task = select_task_inner(&mut tx, task_id, Uuid::nil())
            .await
            .unwrap();
        assert!(task.is_some());

        let task = task.unwrap();

        assert_eq!(task.title, "");
        assert!(task.notes.is_none());
        assert!(task.start.is_none());
        assert!(task.deadline.is_none());
        assert!(task.tags.is_empty());

        assert!(task.deleted_at.is_none());
        assert_eq!(task.created_by, Uuid::nil());
    }

    #[test]
    async fn with_title() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let title = format!("This is a test title for with_title() test");

        let mut task = Task::default();
        task.title = title.clone();

        let task_id = insert_task_inner(&mut tx, Uuid::nil(), task).await.unwrap();

        let task = select_task_inner(&mut tx, task_id, Uuid::nil())
            .await
            .unwrap();
        assert!(task.is_some());

        let task = task.unwrap();

        assert_eq!(task.title, "This is a test title for with_title() test");
        assert!(task.notes.is_none());
        assert!(task.start.is_none());
        assert!(task.deadline.is_none());
        assert!(task.tags.is_empty());

        assert!(task.deleted_at.is_none());
        assert_eq!(task.created_by, Uuid::nil());
    }

    #[test]
    async fn with_notes() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let notes = format!("This is the notes section in the with_notes() test");

        let mut task = Task::default();
        task.notes = Some(notes);

        let task_id = insert_task_inner(&mut tx, Uuid::nil(), task).await.unwrap();

        let task = select_task_inner(&mut tx, task_id, Uuid::nil())
            .await
            .unwrap();
        assert!(task.is_some());

        let task = task.unwrap();

        assert_eq!(task.title, "");
        assert!(task.notes.is_some());
        assert_eq!(
            task.notes.unwrap(),
            "This is the notes section in the with_notes() test"
        );
        assert!(task.start.is_none());
        assert!(task.deadline.is_none());
        assert!(task.tags.is_empty());

        assert!(task.deleted_at.is_none());
        assert_eq!(task.created_by, Uuid::nil());
    }

    #[test]
    async fn with_start_date() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let date = Utc::now().date_naive();

        let mut task = Task::default();
        task.start = Some(Start::On(date.clone()));

        let task_id = insert_task_inner(&mut tx, Uuid::nil(), task).await.unwrap();

        let task = select_task_inner(&mut tx, task_id, Uuid::nil())
            .await
            .unwrap();
        assert!(task.is_some());

        let task = task.unwrap();

        assert_eq!(task.title, "");
        assert!(task.notes.is_none());
        assert!(task.start.is_some());
        assert_eq!(task.start.unwrap(), Start::On(date));
        assert!(task.deadline.is_none());
        assert!(task.tags.is_empty());

        assert!(task.deleted_at.is_none());
        assert_eq!(task.created_by, Uuid::nil());
    }

    #[test]
    async fn with_start_datetime() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let datetime = Utc::now();

        let mut task = Task::default();
        task.start = Some(Start::At(datetime));

        let task_id = insert_task_inner(&mut tx, Uuid::nil(), task).await.unwrap();

        let task = select_task_inner(&mut tx, task_id, Uuid::nil())
            .await
            .unwrap();
        assert!(task.is_some());

        let task = task.unwrap();

        assert_eq!(task.title, "");
        assert!(task.notes.is_none());
        assert!(task.start.is_some());
        assert_eq!(
            task.start.unwrap(),
            Start::At(datetime.trunc_subsecs(PG_SUBSEC_PREC))
        );
        assert!(task.deadline.is_none());
        assert!(task.tags.is_empty());

        assert!(task.deleted_at.is_none());
        assert_eq!(task.created_by, Uuid::nil());
    }

    #[test]
    async fn with_deadline() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let date = Utc::now().date_naive();

        let mut task = Task::default();
        task.deadline = Some(date.clone());

        let task_id = insert_task_inner(&mut tx, Uuid::nil(), task).await.unwrap();

        let task = select_task_inner(&mut tx, task_id, Uuid::nil())
            .await
            .unwrap();
        assert!(task.is_some());

        let task = task.unwrap();

        assert_eq!(task.title, "");
        assert!(task.notes.is_none());
        assert!(task.start.is_none());
        assert!(task.deadline.is_some());
        assert_eq!(task.deadline.unwrap(), date);
        assert!(task.tags.is_empty());

        assert!(task.deleted_at.is_none());
        assert_eq!(task.created_by, Uuid::nil());
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

        let mut task = Task::default();
        task.title = title.clone();
        task.notes = Some(notes.clone());
        task.start = Some(Start::On(start.clone()));
        task.deadline = Some(deadline.clone());

        let task_id = insert_task_inner(&mut tx, Uuid::nil(), task).await.unwrap();

        let task = select_task_inner(&mut tx, task_id, Uuid::nil())
            .await
            .unwrap();
        assert!(task.is_some());

        let task = task.unwrap();

        assert_eq!(task.title, "Homework 1");
        assert!(task.notes.is_some());
        assert_eq!(task.notes.unwrap(), notes);
        assert!(task.start.is_some());
        assert_eq!(task.start.unwrap(), Start::On(start));
        assert!(task.deadline.is_some());
        assert_eq!(task.deadline.unwrap(), deadline);
        assert!(task.tags.is_empty());

        assert!(task.deleted_at.is_none());
        assert_eq!(task.created_by, Uuid::nil());
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

        let mut task = Task::default();
        task.title = title.clone();
        task.notes = Some(notes.clone());
        task.start = Some(Start::At(start_datetime));
        task.deadline = Some(deadline.clone());

        let task_id = insert_task_inner(&mut tx, Uuid::nil(), task).await.unwrap();

        let task = select_task_inner(&mut tx, task_id, Uuid::nil())
            .await
            .unwrap();
        assert!(task.is_some());

        let task = task.unwrap();

        assert_eq!(task.title, "Study for Exam 1");
        assert!(task.notes.is_some());
        assert_eq!(task.notes.unwrap(), notes);
        assert!(task.start.is_some());
        assert_eq!(
            task.start.unwrap(),
            Start::At(start_datetime.trunc_subsecs(PG_SUBSEC_PREC))
        );
        assert!(task.deadline.is_some());
        assert_eq!(task.deadline.unwrap(), deadline);
        assert!(task.tags.is_empty());

        assert!(task.deleted_at.is_none());
        assert_eq!(task.created_by, Uuid::nil());
    }
}

#[cfg(test)]
mod retrieve_tests {
    use std::{collections::HashSet, env, path::Path, time::Duration};

    use chrono::{DateTime, Utc};
    use sqlx::{Connection, PgConnection, PgPool, migrate::Migrator, postgres::PgPoolOptions};
    use tokio::{sync::OnceCell, test};
    use uuid::Uuid;

    use crate::com::model::Task;

    use super::select_task_inner;

    async fn insert_test_task(
        tx: &mut PgConnection,
        task: Task,
        completed_at: Option<DateTime<Utc>>,
        deleted_at: Option<DateTime<Utc>>,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Uuid {
        sqlx::query_scalar(
            "INSERT INTO app.tasks (title, notes, start_on, start_at, deadline, created_by, completed_at, deleted_at, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10) RETURNING id",
        )
        .bind(task.title)
        .bind(task.notes)
        .bind(task.start.as_ref().map(|s| s.as_on()))
        .bind(task.start.as_ref().map(|s| s.as_at()))
        .bind(task.deadline)
        .bind(Uuid::nil())
        .bind(completed_at)
        .bind(deleted_at)
        .bind(created_at)
        .bind(updated_at)
        .fetch_one(tx)
        .await.unwrap()
    }

    static POOL: OnceCell<PgPool> = OnceCell::const_new();
    async fn get_pool() -> &'static PgPool {
        POOL.get_or_init(|| async {
            dotenvy::from_filename("./.env.testing").ok();

            // migration
            let user = env::var("MIGRATION_USER").unwrap();
            let pass = env::var("MIGRATION_PASS").unwrap();
            let db = env::var("MIGRATION_DB").unwrap();
            let url = format!("postgresql://{}:{}@localhost/{}", user, pass, db);

            let mut conn = PgConnection::connect(&url).await.unwrap();

            let m = Migrator::new(Path::new("./migrations")).await.unwrap();
            m.run(&mut conn).await.unwrap();

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
            let pool = pool_opts.connect(&url).await.unwrap();

            pool
        })
        .await
    }

    #[test]
    async fn base_select() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = Utc::now();

        let task_id = insert_test_task(
            &mut tx,
            Task::default(),
            None,
            None,
            base_time,
            base_time + Duration::from_hours(1),
        )
        .await;

        let task = select_task_inner(&mut tx, task_id, Uuid::nil())
            .await
            .unwrap();
        assert!(task.is_some());

        let task = task.unwrap();
        assert_eq!(task.title, "");
        assert!(task.notes.is_none());
        assert!(task.start.is_none());
        assert!(task.deadline.is_none());
        assert!(task.tags.is_empty());
        assert!(task.completed_at.is_none());
        assert!(task.deleted_at.is_none());
    }

    #[test]
    async fn select_many_different() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = Utc::now();

        let mut seen_ids = HashSet::new();
        for _ in 0..10 {
            let task_id = insert_test_task(
                &mut tx,
                Task::default(),
                None,
                None,
                base_time,
                base_time + Duration::from_hours(1),
            )
            .await;

            assert!(seen_ids.insert(task_id), "duplicate task encountered");
        }
        assert!(!seen_ids.is_empty());

        for id in seen_ids {
            let task = select_task_inner(&mut tx, id, Uuid::nil()).await.unwrap();
            assert!(task.is_some());

            let task = task.unwrap();
            assert_eq!(task.title, "");
            assert!(task.notes.is_none());
            assert!(task.start.is_none());
            assert!(task.deadline.is_none());
            assert!(task.tags.is_empty());
            assert!(task.completed_at.is_none());
            assert!(task.deleted_at.is_none());
        }
    }

    #[test]
    async fn try_select_deleted() {
        let pool = get_pool().await;
        let mut tx = pool.begin().await.unwrap();

        let base_time = Utc::now();

        let mut seen_ids = HashSet::new();
        for _ in 0..10 {
            let task_id = insert_test_task(
                &mut tx,
                Task::default(),
                None,
                Some(base_time + Duration::from_hours(1)),
                base_time,
                base_time + Duration::from_hours(1),
            )
            .await;

            assert!(seen_ids.insert(task_id), "duplicate task encountered");
        }
        assert!(!seen_ids.is_empty());

        for id in seen_ids {
            let task = select_task_inner(&mut tx, id, Uuid::nil()).await.unwrap();
            assert!(task.is_none());
        }
    }
}

#[cfg(test)]
mod update_tests {}

#[cfg(test)]
mod delete_tests {}
