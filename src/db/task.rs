use std::collections::{HashMap, hash_map::Entry};

use sqlx::{PgPool, Postgres, QueryBuilder, Transaction};
use uuid::Uuid;

use crate::{
    com::model::{
        Tag, Task, TaskIntermediate, TaskTag,
        db::{DateFilterDB, SQLCmp, SortOrder},
    },
    db::{DEFAULT_LIMIT, MAX_LIMIT, query_as_wrapper},
};

use super::{Error, Result};

pub struct TaskQueryOptions {
    pub user_id: Uuid,

    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub deleted: bool,
    // TODO: This has weird behavior in different timezones... how to resolve
    pub start_filter: Option<DateFilterDB>,
    pub deadline_filter: Option<DateFilterDB>,
    pub sort_order: SortOrder,
}

pub async fn query_tasks(conn: PgPool, opts: TaskQueryOptions) -> Result<Vec<Task>> {
    let limit = opts.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
    let offset = opts.offset.unwrap_or(0).max(0);

    let mut builder = QueryBuilder::new("SELECT * FROM app.tasks WHERE created_by = ");
    builder.push_bind(opts.user_id);
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

    let mut tasks = query.fetch_all(&conn).await?;

    let task_ids: Vec<Uuid> = tasks.iter().map(|task| task.id).collect();
    let tags = query_as_wrapper::<TaskTag>(
        "SELECT tt.task_id, tg.*
            FROM app.task_tags tt
            LEFT JOIN app.tags tg ON tt.tag_id = tg.id
            WHERE tt.task_id = ANY($1)",
    )
    .bind(task_ids)
    .fetch_all(&conn)
    .await?;

    let mut task_tag_map: HashMap<Uuid, Vec<Tag>> = HashMap::new();
    for TaskTag { task_id, tag } in tags {
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
    let mut transaction = conn.begin().await?;

    let task_id = sqlx::query_scalar(
        "INSERT INTO app.tasks (title, notes, start_on, start_at, deadline, created_by)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING id",
    )
    .bind(task.title)
    .bind(task.notes)
    .bind(task.start.as_on())
    .bind(task.start.as_at())
    .bind(task.deadline)
    .bind(user_id)
    .fetch_one(&mut *transaction)
    .await?;

    if !task.tags.is_empty() {
        update_tag_helper(&mut transaction, task_id, task.tags).await?;
    }

    transaction.commit().await?;

    Ok(task_id)
}

pub async fn select_task(conn: PgPool, task_id: Uuid, user_id: Uuid) -> Result<Option<Task>> {
    let task_opt = query_as_wrapper::<TaskIntermediate>(
        "SELECT *
        FROM app.tasks
        WHERE id = $1 AND created_by = $2 AND deleted_at IS NULL",
    )
    .bind(task_id)
    .bind(user_id)
    .fetch_optional(&conn)
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
            .fetch_all(&conn)
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
    let mut transaction = conn.begin().await?;

    if sqlx::query(
        "UPDATE app.tasks SET
        (updated_at, title, notes, start_on, start_at, deadline) =
        (CURRENT_TIMESTAMP, $3, $4, $5, $6, $7)
        WHERE id = $1 AND created_by = $2 AND deleted_at IS NULL",
    )
    .bind(task_id)
    .bind(user_id)
    .bind(task.title)
    .bind(task.notes)
    .bind(task.start.as_on())
    .bind(task.start.as_at())
    .bind(task.deadline)
    .execute(&mut *transaction)
    .await?
    .rows_affected()
        == 0
    {
        return Ok(None);
    }

    if !task.tags.is_empty() {
        update_tag_helper(&mut transaction, task_id, task.tags).await?;
    }

    transaction.commit().await?;

    select_task(conn, task_id, user_id).await
}

pub async fn delete_task(conn: PgPool, task_id: Uuid, user_id: Uuid) -> Result<Option<()>> {
    let mut transaction = conn.begin().await?;

    if sqlx::query(
        "UPDATE app.tasks SET
        (updated_at, deleted_at) =
        (CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
        WHERE id = $1 AND created_by = $2 AND deleted_at IS NULL",
    )
    .bind(task_id)
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

pub async fn update_tag_helper(
    tx: &mut Transaction<'_, Postgres>,
    task_id: Uuid,
    tags: Vec<Tag>,
) -> Result<()> {
    let mut builder = QueryBuilder::new("INSERT INTO app.task_tags (task_id, tag_id) VALUES");

    let mut separated = builder.separated(", ");
    for tag in tags.iter() {
        separated.push(" (");
        separated.push_bind(task_id);
        separated.push(", ");
        separated.push_bind(tag.id);
        separated.push(")");
    }

    let num_rows = builder.build().execute(&mut **tx).await?.rows_affected() as usize;

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
    use std::collections::HashSet;
    use std::env;
    use std::sync::OnceLock;

    use chrono::NaiveDate;
    use tokio::test;

    use crate::com::model::db::DateBound;
    use crate::com::model::util::Start;

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

    fn create_default_opts() -> TaskQueryOptions {
        TaskQueryOptions {
            user_id: Uuid::nil(),
            limit: None,
            offset: None,
            deleted: false,
            start_filter: None,
            deadline_filter: None,
            sort_order: SortOrder::default(),
        }
    }

    #[test]
    async fn ensure_data_in_database() {
        let conn = setup().await;

        let opts = create_default_opts();
        let tasks = query_tasks(conn.clone(), opts).await.unwrap();
        assert_ne!(tasks.len(), 0, "no data in database setup for testing");

        let mut opts = create_default_opts();
        opts.deleted = true;
        let tasks = query_tasks(conn, opts).await.unwrap();
        assert_ne!(tasks.len(), 0, "no deleted database setup for testing");
    }

    #[test]
    async fn base_query() {
        let conn = setup().await;

        let opts = create_default_opts();

        let tasks = query_tasks(conn, opts).await.unwrap();

        // default sort is updated_at descending
        assert!(tasks.is_sorted_by(|a, b| a.updated_at >= b.updated_at));

        for task in &tasks {
            // no deleted tasks
            assert!(task.deleted_at.is_none());
        }
    }

    #[test]
    async fn updated_by_ascending() {
        let conn = setup().await;

        let mut opts = create_default_opts();
        opts.sort_order = SortOrder::UpdatedAsc;

        let tasks = query_tasks(conn, opts).await.unwrap();

        // default sort is updated_at descending
        assert!(tasks.is_sorted_by(|a, b| a.updated_at <= b.updated_at));

        for task in &tasks {
            // no deleted tasks
            assert!(task.deleted_at.is_none());
        }
    }

    #[test]
    async fn created_by_descending() {
        let conn = setup().await;

        let mut opts = create_default_opts();
        opts.sort_order = SortOrder::CreatedDesc;

        let tasks = query_tasks(conn, opts).await.unwrap();

        // default sort is updated_at descending
        assert!(tasks.is_sorted_by(|a, b| a.created_at >= b.created_at));

        for task in &tasks {
            // no deleted tasks
            assert!(task.deleted_at.is_none());
        }
    }

    #[test]
    async fn created_by_ascending() {
        let conn = setup().await;

        let mut opts = create_default_opts();
        opts.sort_order = SortOrder::CreatedAsc;

        let tasks = query_tasks(conn, opts).await.unwrap();

        // default sort is updated_at descending
        assert!(tasks.is_sorted_by(|a, b| a.created_at <= b.created_at));

        for task in &tasks {
            // no deleted tasks
            assert!(task.deleted_at.is_none());
        }
    }

    #[test]
    async fn limit() {
        let conn = setup().await;

        for i in 1..=10 {
            let mut opts = create_default_opts();
            opts.limit = Some(i);

            let tasks = query_tasks(conn.clone(), opts).await.unwrap();

            assert_eq!(tasks.len() as i64, i);

            for task in &tasks {
                // no deleted tasks
                assert!(task.deleted_at.is_none());
            }
        }
    }

    #[test]
    async fn limit_0() {
        let conn = setup().await;

        let mut opts = create_default_opts();
        opts.limit = Some(0);

        query_tasks(conn, opts).await.unwrap();
    }

    #[test]
    async fn limit_negative() {
        let conn = setup().await;

        let mut opts = create_default_opts();
        opts.limit = Some(-10);

        query_tasks(conn, opts).await.unwrap();
    }

    #[test]
    async fn limit_with_paging_offset() {
        let conn = setup().await;

        let limit = 5;

        // keep paging until less than 'limit' tasks are return
        let mut i = 0;
        let mut seen = HashSet::new();
        loop {
            let mut opts = create_default_opts();
            opts.limit = Some(limit);
            opts.offset = Some(i * limit);

            let tasks = query_tasks(conn.clone(), opts).await.unwrap();

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

        let tasks = query_tasks(conn, opts).await.unwrap();

        assert_eq!(tasks.len(), 0);
    }

    #[test]
    async fn offset_negative() {
        let conn = setup().await;

        let mut opts = create_default_opts();
        opts.offset = Some(-10);

        query_tasks(conn, opts).await.unwrap();
    }

    #[test]
    async fn offset_without_limits() {
        let conn = setup().await;

        let mut opts = create_default_opts();
        opts.offset = Some(20);

        query_tasks(conn, opts).await.unwrap();
    }

    #[test]
    async fn get_deleted() {
        let conn = setup().await;

        let mut opts = create_default_opts();
        opts.deleted = true;

        let tasks = query_tasks(conn, opts).await.unwrap();

        for task in &tasks {
            assert!(task.deleted_at.is_some());
        }
    }

    #[test]
    async fn filter_start_on() {
        let conn = setup().await;

        let test_date = NaiveDate::from_ymd_opt(2025, 1, 2).unwrap();

        let mut opts = create_default_opts();
        opts.start_filter = Some(DateFilterDB::On(test_date));

        let tasks = query_tasks(conn, opts).await.unwrap();

        for task in &tasks {
            // no deleted tasks
            assert!(task.deleted_at.is_none());

            assert!(!matches!(task.start, Start::None));

            match task.start {
                Start::On(date) => assert_eq!(date, test_date),
                Start::At(datetime) => assert_eq!(datetime.date_naive(), test_date),
                _ => unreachable!(),
            }
        }
    }

    #[test]
    async fn filter_start_not_on() {
        let conn = setup().await;

        let test_date = NaiveDate::from_ymd_opt(2025, 1, 2).unwrap();

        let mut opts = create_default_opts();
        opts.start_filter = Some(DateFilterDB::NotOn(test_date));

        let tasks = query_tasks(conn, opts).await.unwrap();

        for task in &tasks {
            // no deleted tasks
            assert!(task.deleted_at.is_none());

            match task.start {
                Start::On(date) => assert_ne!(date, test_date),
                Start::At(datetime) => assert_ne!(datetime.date_naive(), test_date),
                _ => (),
            }
        }
    }

    #[test]
    async fn filter_start_after_excl() {
        let conn = setup().await;

        let test_date = NaiveDate::from_ymd_opt(2025, 1, 2).unwrap();

        let mut opts = create_default_opts();
        opts.start_filter = Some(DateFilterDB::StartRange(DateBound::Exclusive(test_date)));

        let tasks = query_tasks(conn, opts).await.unwrap();

        for task in &tasks {
            // no deleted tasks
            assert!(task.deleted_at.is_none());

            assert!(!matches!(task.start, Start::None));

            match task.start {
                Start::On(date) => assert!(date > test_date),
                Start::At(datetime) => assert!(datetime.date_naive() > test_date),
                _ => unreachable!(),
            }
        }
    }

    #[test]
    async fn filter_start_after_incl() {
        let conn = setup().await;

        let test_date = NaiveDate::from_ymd_opt(2025, 1, 2).unwrap();

        let mut opts = create_default_opts();
        opts.start_filter = Some(DateFilterDB::StartRange(DateBound::Inclusive(test_date)));

        let tasks = query_tasks(conn, opts).await.unwrap();

        for task in &tasks {
            // no deleted tasks
            assert!(task.deleted_at.is_none());

            assert!(!matches!(task.start, Start::None));

            match task.start {
                Start::On(date) => assert!(date >= test_date),
                Start::At(datetime) => assert!(datetime.date_naive() >= test_date),
                _ => unreachable!(),
            }
        }
    }

    #[test]
    async fn filter_start_before_excl() {
        let conn = setup().await;

        let test_date = NaiveDate::from_ymd_opt(2025, 1, 2).unwrap();

        let mut opts = create_default_opts();
        opts.start_filter = Some(DateFilterDB::EndRange(DateBound::Exclusive(test_date)));

        let tasks = query_tasks(conn, opts).await.unwrap();

        for task in &tasks {
            // no deleted tasks
            assert!(task.deleted_at.is_none());

            assert!(!matches!(task.start, Start::None));

            match task.start {
                Start::On(date) => assert!(date < test_date),
                Start::At(datetime) => assert!(datetime.date_naive() < test_date),
                _ => unreachable!(),
            }
        }
    }

    #[test]
    async fn filter_start_before_incl() {
        let conn = setup().await;

        let test_date = NaiveDate::from_ymd_opt(2025, 1, 2).unwrap();

        let mut opts = create_default_opts();
        opts.start_filter = Some(DateFilterDB::EndRange(DateBound::Inclusive(test_date)));

        let tasks = query_tasks(conn, opts).await.unwrap();

        for task in &tasks {
            // no deleted tasks
            assert!(task.deleted_at.is_none());

            assert!(!matches!(task.start, Start::None));

            match task.start {
                Start::On(date) => assert!(date <= test_date),
                Start::At(datetime) => assert!(datetime.date_naive() <= test_date),
                _ => unreachable!(),
            }
        }
    }

    #[test]
    async fn filter_start_between_excl() {
        let conn = setup().await;

        let test_date_min = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let test_date_max = NaiveDate::from_ymd_opt(2025, 1, 4).unwrap();

        let mut opts = create_default_opts();
        opts.start_filter = Some(DateFilterDB::Range(
            DateBound::Exclusive(test_date_min),
            DateBound::Exclusive(test_date_max),
        ));

        let tasks = query_tasks(conn, opts).await.unwrap();

        for task in &tasks {
            // no deleted tasks
            assert!(task.deleted_at.is_none());

            assert!(!matches!(task.start, Start::None));

            match task.start {
                Start::On(date) => assert!(date > test_date_min && date < test_date_max),
                Start::At(datetime) => assert!(
                    datetime.date_naive() > test_date_min && datetime.date_naive() < test_date_max
                ),
                _ => unreachable!(),
            }
        }
    }

    #[test]
    async fn filter_start_between_incl() {
        let conn = setup().await;

        let test_date_min = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let test_date_max = NaiveDate::from_ymd_opt(2025, 1, 4).unwrap();

        let mut opts = create_default_opts();
        opts.start_filter = Some(DateFilterDB::Range(
            DateBound::Inclusive(test_date_min),
            DateBound::Inclusive(test_date_max),
        ));

        let tasks = query_tasks(conn, opts).await.unwrap();

        for task in &tasks {
            // no deleted tasks
            assert!(task.deleted_at.is_none());

            assert!(!matches!(task.start, Start::None));

            match task.start {
                Start::On(date) => assert!(date >= test_date_min && date <= test_date_max),
                Start::At(datetime) => assert!(
                    datetime.date_naive() >= test_date_min
                        && datetime.date_naive() <= test_date_max
                ),
                _ => unreachable!(),
            }
        }
    }

    #[test]
    async fn filter_deadline_on() {
        let conn = setup().await;

        let test_date = NaiveDate::from_ymd_opt(2025, 1, 4).unwrap();

        let mut opts = create_default_opts();
        opts.deadline_filter = Some(DateFilterDB::On(test_date));

        let tasks = query_tasks(conn, opts).await.unwrap();

        for task in &tasks {
            // no deleted tasks
            assert!(task.deleted_at.is_none());

            assert_eq!(task.deadline, Some(test_date));
        }
    }

    #[test]
    async fn filter_deadline_not_on() {
        let conn = setup().await;

        let test_date = NaiveDate::from_ymd_opt(2025, 1, 4).unwrap();

        let mut opts = create_default_opts();
        opts.deadline_filter = Some(DateFilterDB::NotOn(test_date));

        let tasks = query_tasks(conn, opts).await.unwrap();

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
        let conn = setup().await;

        let test_date = NaiveDate::from_ymd_opt(2025, 1, 4).unwrap();

        let mut opts = create_default_opts();
        opts.deadline_filter = Some(DateFilterDB::StartRange(DateBound::Exclusive(test_date)));

        let tasks = query_tasks(conn, opts).await.unwrap();

        for task in &tasks {
            // no deleted tasks
            assert!(task.deleted_at.is_none());

            assert!(!matches!(task.deadline, None));

            assert!(task.deadline.unwrap() > test_date);
        }
    }

    #[test]
    async fn filter_deadline_after_incl() {
        let conn = setup().await;

        let test_date = NaiveDate::from_ymd_opt(2025, 1, 4).unwrap();

        let mut opts = create_default_opts();
        opts.deadline_filter = Some(DateFilterDB::StartRange(DateBound::Inclusive(test_date)));

        let tasks = query_tasks(conn, opts).await.unwrap();

        for task in &tasks {
            // no deleted tasks
            assert!(task.deleted_at.is_none());

            assert!(!matches!(task.deadline, None));

            assert!(task.deadline.unwrap() >= test_date);
        }
    }

    #[test]
    async fn filter_deadline_before_excl() {
        let conn = setup().await;

        let test_date = NaiveDate::from_ymd_opt(2025, 1, 4).unwrap();

        let mut opts = create_default_opts();
        opts.deadline_filter = Some(DateFilterDB::EndRange(DateBound::Exclusive(test_date)));

        let tasks = query_tasks(conn, opts).await.unwrap();

        for task in &tasks {
            // no deleted tasks
            assert!(task.deleted_at.is_none());

            assert!(!matches!(task.deadline, None));

            assert!(task.deadline.unwrap() < test_date);
        }
    }

    #[test]
    async fn filter_deadline_before_incl() {
        let conn = setup().await;

        let test_date = NaiveDate::from_ymd_opt(2025, 1, 4).unwrap();

        let mut opts = create_default_opts();
        opts.deadline_filter = Some(DateFilterDB::EndRange(DateBound::Inclusive(test_date)));

        let tasks = query_tasks(conn, opts).await.unwrap();

        for task in &tasks {
            // no deleted tasks
            assert!(task.deleted_at.is_none());

            assert!(!matches!(task.deadline, None));

            assert!(task.deadline.unwrap() <= test_date);
        }
    }

    #[test]
    async fn filter_deadline_between_excl() {
        let conn = setup().await;

        let test_date_min = NaiveDate::from_ymd_opt(2025, 1, 3).unwrap();
        let test_date_max = NaiveDate::from_ymd_opt(2025, 1, 6).unwrap();

        let mut opts = create_default_opts();
        opts.deadline_filter = Some(DateFilterDB::Range(
            DateBound::Exclusive(test_date_min),
            DateBound::Exclusive(test_date_max),
        ));

        let tasks = query_tasks(conn, opts).await.unwrap();

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
        let conn = setup().await;

        let test_date_min = NaiveDate::from_ymd_opt(2025, 1, 3).unwrap();
        let test_date_max = NaiveDate::from_ymd_opt(2025, 1, 6).unwrap();

        let mut opts = create_default_opts();
        opts.deadline_filter = Some(DateFilterDB::Range(
            DateBound::Inclusive(test_date_min),
            DateBound::Inclusive(test_date_max),
        ));

        let tasks = query_tasks(conn, opts).await.unwrap();

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
        let conn = setup().await;

        let mut opts = create_default_opts();
        opts.user_id = Uuid::new_v4();

        let tasks = query_tasks(conn, opts).await.unwrap();

        assert_eq!(tasks.len(), 0);
    }
}

#[cfg(test)]
mod create_tests {}

#[cfg(test)]
mod retrieve_tests {}

#[cfg(test)]
mod update_tests {}

#[cfg(test)]
mod delete_tests {}
