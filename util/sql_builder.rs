use std::fmt::Write;

use tokio_postgres::types::ToSql;

pub const NULL: Option<String> = None;

#[allow(dead_code)]
pub enum PostgresCmp {
    Equal,
    NotEqual,
    Less,
    LessEq,
    Greater,
    GreaterEq,
    IsNull,
    NotNull,
    Like,
    ILike,
    In,
    NotIn,
}

impl PostgresCmp {
    pub fn as_sql_cmp(&self) -> &str {
        match self {
            PostgresCmp::Equal => "=",
            PostgresCmp::NotEqual => "<>",
            PostgresCmp::Less => "<",
            PostgresCmp::LessEq => "<=",
            PostgresCmp::Greater => ">",
            PostgresCmp::GreaterEq => ">=",
            _ => panic!(),
        }
    }
}

pub trait ToPostgresCmp {
    fn to_postgres_cmp(&self) -> PostgresCmp;
}

/// Postgres joins
#[allow(dead_code)]
pub enum Join {
    Inner,
    Left,
    Right,
    Full,
}

/// SQL Query Builder
///
/// This may/should become its own crate
/// (most likely for personal use only)
#[derive(Default)]
pub struct SqlQueryBuilder<'a> {
    table: String,
    join_tables: Vec<(Join, String, String)>,
    columns: Vec<(String, &'a (dyn ToSql + Sync))>,
    conditions: Vec<(String, PostgresCmp, &'a (dyn ToSql + Sync))>,
    group_by: Vec<String>,
    having: Vec<(String, PostgresCmp, &'a (dyn ToSql + Sync))>,
    limit: Option<usize>,
    offset: Option<usize>,
    return_columns: Vec<String>,
}

impl<'a> SqlQueryBuilder<'a> {
    pub fn new(table_name: &str) -> Self {
        Self {
            table: table_name.to_string(),
            ..Default::default()
        }
    }

    /// Add tables to join
    pub fn add_join(&mut self, join_type: Join, table_name: &str, join_column: &str) -> &mut Self {
        self.join_tables
            .push((join_type, table_name.to_string(), join_column.to_string()));

        self
    }

    /// Add value for a given column
    pub fn add_column(&mut self, column: &str, value: &'a (dyn ToSql + Sync)) -> &mut Self {
        self.columns.push((column.to_string(), value));

        self
    }

    /// Adds a condition to query
    ///
    /// If cmp is an operator that does not require a value, then the value is ignored.
    /// However, a value must be included.
    ///
    /// A `NULL` value is provided in the crate for this purpose.
    pub fn add_condition(
        &mut self,
        column: &str,
        cmp: PostgresCmp,
        value: &'a (dyn ToSql + Sync),
    ) -> &mut Self {
        self.conditions.push((column.to_string(), cmp, value));

        self
    }

    pub fn set_group_by(&mut self, columns: &[&str]) -> &mut Self {
        self.group_by = columns.iter().map(|s| s.to_string()).collect();

        self
    }

    pub fn set_having(
        &mut self,
        item: &str,
        cmp: PostgresCmp,
        value: &'a (dyn ToSql + Sync),
    ) -> &mut Self {
        self.having.push((item.to_string(), cmp, value));

        self
    }

    /// Set limit for query
    pub fn set_limit(&mut self, limit: usize) -> &mut Self {
        self.limit = Some(limit);

        self
    }

    /// Set offset for query
    pub fn set_offset(&mut self, offset: usize) -> &mut Self {
        self.offset = Some(offset);

        self
    }

    /// Set query to return columns specified
    ///
    /// When using this with a `SELECT` query, limits columns returned
    pub fn set_return(&mut self, columns: &[&str]) -> &mut Self {
        self.return_columns = columns.iter().map(|s| s.to_string()).collect();

        self
    }

    /// Sets query to return all columns
    ///
    /// Using this when creating a `SELECT` query returns select back to default
    /// of returning all columns.
    ///
    /// For other queries, returns all columns
    pub fn set_return_all(&mut self) -> &mut Self {
        self.return_columns.clear();
        self.return_columns.push("*".to_string());

        self
    }
}

impl<'a> SqlQueryBuilder<'a> {
    /// Builds an SQL SELECT query
    ///
    /// Consumes query builder after use
    pub fn build_select(self) -> (String, Vec<&'a (dyn ToSql + Sync)>) {
        let mut query: String = String::from("SELECT");
        let mut param_n = 1;
        let mut params: Vec<&(dyn ToSql + Sync)> = Vec::new();

        // Add columns to retrieve
        if self.return_columns.is_empty() || self.return_columns[0].eq("*") {
            query.push_str(" *");
        } else {
            query.push(' ');
            query.push_str(&self.return_columns.join(", "));
        }

        query.push_str(" FROM ");
        query.push_str(&self.table);

        // Add any joins
        for (join_type, table, col) in self.join_tables {
            match join_type {
                Join::Inner => write!(query, " INNER JOIN").unwrap(),
                Join::Left => write!(query, " LEFT JOIN").unwrap(),
                Join::Right => write!(query, " RIGHT JOIN").unwrap(),
                Join::Full => write!(query, " FULL JOIN").unwrap(),
            }

            write!(query, " {table} USING ({col})").unwrap();
        }

        // Add any conditions to select
        if !self.conditions.is_empty() {
            query.push_str(" WHERE ");

            for (n, (col, cmp, val)) in self.conditions.iter().enumerate() {
                if n > 0 {
                    query.push_str(" AND ");
                }

                match cmp {
                    PostgresCmp::IsNull => write!(query, "{col} IS NULL").unwrap(),
                    PostgresCmp::NotNull => write!(query, "{col} NOT NULL").unwrap(),
                    PostgresCmp::Like => {
                        write!(query, "{col} LIKE '%' || ${param_n} || '%'").unwrap();
                        param_n += 1;
                        params.push(val.to_owned());
                    }
                    PostgresCmp::ILike => {
                        write!(query, "{col} ILIKE '%' || ${param_n} || '%'").unwrap();
                        param_n += 1;
                        params.push(val.to_owned());
                    }
                    PostgresCmp::In => {
                        write!(query, "{col} = ANY(${param_n})").unwrap();
                        param_n += 1;
                        params.push(val.to_owned());
                    }
                    PostgresCmp::NotIn => {
                        write!(query, "{col} != ALL(${param_n})").unwrap();
                        param_n += 1;
                        params.push(val.to_owned());
                    }
                    _ => {
                        write!(query, "{col} {} ${param_n}", cmp.as_sql_cmp()).unwrap();
                        param_n += 1;
                        params.push(val.to_owned());
                    }
                }
            }
        }

        // Add group by
        if !self.group_by.is_empty() {
            query.push_str(" GROUP BY ");
            query.push_str(&self.group_by.join(", "));
        }

        // Add having
        if !self.having.is_empty() {
            query.push_str(" HAVING ");

            for (n, (item, cmp, val)) in self.having.iter().enumerate() {
                if n > 0 {
                    query.push_str(" AND ");
                }

                write!(query, "{item} {} ${param_n}", cmp.as_sql_cmp()).unwrap();
                param_n += 1;
                params.push(val.to_owned());
            }
        }

        // Add limit
        if let Some(limit) = self.limit {
            write!(query, " LIMIT {limit}").unwrap();
        }

        // Add offset
        if let Some(offset) = self.offset {
            write!(query, " OFFSET {offset}").unwrap();
        }

        (query, params)
    }

    /// Builds an SQL INSERT query
    ///
    /// Consumes query builder after use
    pub fn build_insert(self) -> (String, Vec<&'a (dyn ToSql + Sync)>) {
        let mut query: String = String::from("INSERT");
        let mut param_n = 1;
        let mut params: Vec<&(dyn ToSql + Sync)> = Vec::new();

        query.push_str(" INTO ");
        query.push_str(&self.table);

        // Add columns to get from
        if self.columns.is_empty() {
            panic!();
        }
        let (col, val): (Vec<String>, Vec<&'a (dyn ToSql + Sync)>) =
            self.columns.iter().cloned().unzip();
        query.push_str(" (");
        query.push_str(&col.join(", "));
        query.push_str(") VALUES (");
        for (n, val) in val.iter().enumerate() {
            if n > 0 {
                query.push_str(", ");
            }

            write!(query, "${param_n}").unwrap();
            param_n += 1;
            params.push(val.to_owned());
        }
        query.push(')');

        // Add returning columns
        if !self.return_columns.is_empty() {
            if self.return_columns[0].eq("*") {
                query.push_str(" RETURNING *");
            } else {
                query.push_str(" RETURNING ");
                query.push_str(&self.return_columns.join(", "));
            }
        }

        (query, params)
    }

    /// Builds an SQL UPDATE query
    ///
    /// Consumes query builder after use
    pub fn build_update(self) -> (String, Vec<&'a (dyn ToSql + Sync)>) {
        let mut query: String = String::from("UPDATE");
        let mut param_n = 1;
        let mut params: Vec<&(dyn ToSql + Sync)> = Vec::new();

        // Add table to update
        query.push(' ');
        query.push_str(&self.table);

        // Add columns to update to
        if self.columns.is_empty() {
            panic!();
        }
        query.push_str(" SET ");
        for (n, (col, val)) in self.columns.iter().enumerate() {
            if n > 0 {
                query.push_str(", ");
            }

            write!(query, "{col}=${param_n}").unwrap();
            param_n += 1;
            params.push(val.to_owned());
        }

        // Add conditions
        if !self.conditions.is_empty() {
            query.push_str(" WHERE ");

            for (n, (col, cmp, val)) in self.conditions.iter().enumerate() {
                if n > 0 {
                    query.push_str(" AND ");
                }

                match cmp {
                    PostgresCmp::IsNull => write!(query, "{col} IS NULL").unwrap(),
                    PostgresCmp::NotNull => write!(query, "{col} NOT NULL").unwrap(),
                    PostgresCmp::Like => {
                        write!(query, "{col} LIKE '%' || ${param_n} || '%'").unwrap();
                        param_n += 1;
                        params.push(val.to_owned());
                    }
                    PostgresCmp::ILike => {
                        write!(query, "{col} ILIKE '%' || ${param_n} || '%'").unwrap();
                        param_n += 1;
                        params.push(val.to_owned());
                    }
                    PostgresCmp::In => {
                        write!(query, "{col} = ANY(${param_n})").unwrap();
                        param_n += 1;
                        params.push(val.to_owned());
                    }
                    PostgresCmp::NotIn => {
                        write!(query, "{col} != ALL(${param_n})").unwrap();
                        param_n += 1;
                        params.push(val.to_owned());
                    }
                    _ => {
                        write!(query, "{col} {} ${param_n}", cmp.as_sql_cmp()).unwrap();
                        param_n += 1;
                        params.push(val.to_owned());
                    }
                }
            }
        }

        // Add returning columns
        if !self.return_columns.is_empty() {
            if self.return_columns[0].eq("*") {
                query.push_str(" RETURNING *");
            } else {
                query.push_str(" RETURNING ");
                query.push_str(&self.return_columns.join(", "));
            }
        }

        (query, params)
    }

    /// Builds an SQL DELETE query
    ///
    /// Consumes query builder after use
    pub fn build_delete(self) -> (String, Vec<&'a (dyn ToSql + Sync)>) {
        let mut query: String = String::from("DELETE");
        let mut param_n = 1;
        let mut params: Vec<&(dyn ToSql + Sync)> = Vec::new();

        // Add table to delete from
        query.push_str(" FROM ");
        query.push_str(&self.table);

        // Add conditions
        if !self.conditions.is_empty() {
            query.push_str(" WHERE ");
            for (n, (col, cmp, val)) in self.conditions.iter().enumerate() {
                if n > 0 {
                    query.push_str(" AND ");
                }

                match cmp {
                    PostgresCmp::IsNull => write!(query, "{col} IS NULL").unwrap(),
                    PostgresCmp::NotNull => write!(query, "{col} NOT NULL").unwrap(),
                    PostgresCmp::Like => {
                        write!(query, "{col} LIKE '%' || ${param_n} || '%'").unwrap();
                        param_n += 1;
                        params.push(val.to_owned());
                    }
                    PostgresCmp::ILike => {
                        write!(query, "{col} ILIKE '%' || ${param_n} || '%'").unwrap();
                        param_n += 1;
                        params.push(val.to_owned());
                    }
                    PostgresCmp::In => {
                        write!(query, "{col} = ANY(${param_n})").unwrap();
                        param_n += 1;
                        params.push(val.to_owned());
                    }
                    PostgresCmp::NotIn => {
                        write!(query, "{col} != ALL(${param_n})").unwrap();
                        param_n += 1;
                        params.push(val.to_owned());
                    }
                    _ => {
                        write!(query, "{col} {} ${param_n}", cmp.as_sql_cmp()).unwrap();
                        param_n += 1;
                        params.push(val.to_owned());
                    }
                }
            }
        }

        // Add returning columns
        if !self.return_columns.is_empty() {
            if self.return_columns[0].eq("*") {
                query.push_str(" RETURNING *");
            } else {
                query.push_str(" RETURNING ");
                query.push_str(&self.return_columns.join(", "));
            }
        }

        (query, params)
    }
}

pub trait ToSqlQueryBuilder {
    fn to_sql_builder(&self) -> SqlQueryBuilder<'_>;
}

#[cfg(test)]
mod select_builder {
    use super::{Join, NULL, PostgresCmp, SqlQueryBuilder};

    #[test]
    fn empty() {
        let builder = SqlQueryBuilder::new("table");

        let (statement, params) = builder.build_select();

        assert_eq!(statement.as_str(), "SELECT * FROM table");
        assert_eq!(params.len(), 0);
    }

    #[test]
    fn one_column() {
        let mut builder = SqlQueryBuilder::new("table");

        builder.set_return(&["col_1"]);

        let (statement, params) = builder.build_select();

        assert_eq!(statement.as_str(), "SELECT col_1 FROM table");
        assert_eq!(params.len(), 0);
    }

    #[test]
    fn many_columns() {
        let mut builder = SqlQueryBuilder::new("table");

        builder.set_return(&["col_1", "col_2", "col_3"]);

        let (statement, params) = builder.build_select();

        assert_eq!(statement.as_str(), "SELECT col_1, col_2, col_3 FROM table");
        assert_eq!(params.len(), 0);
    }

    #[test]
    fn all_columns() {
        let mut builder = SqlQueryBuilder::new("table");

        builder.set_return_all();

        let (statement, params) = builder.build_select();

        assert_eq!(statement.as_str(), "SELECT * FROM table");
        assert_eq!(params.len(), 0);
    }

    #[test]
    fn one_condition() {
        let mut builder = SqlQueryBuilder::new("table");

        let val_1: i32 = 10;
        builder.add_condition("col_1", PostgresCmp::Less, &val_1);

        let (statement, params) = builder.build_select();

        assert_eq!(statement.as_str(), "SELECT * FROM table WHERE col_1 < $1");
        assert_eq!(params.len(), 1);
    }

    #[test]
    fn many_conditions() {
        let mut builder = SqlQueryBuilder::new("table");

        let val_1: i32 = 10;
        let val_2: i32 = 35;
        builder.add_condition("col_1", PostgresCmp::Less, &val_1);
        builder.add_condition("col_2", PostgresCmp::Equal, &val_2);
        builder.add_condition("col_3", PostgresCmp::IsNull, &NULL);

        let (statement, params) = builder.build_select();

        assert_eq!(
            statement.as_str(),
            "SELECT * FROM table WHERE col_1 < $1 AND col_2 = $2 AND col_3 IS NULL"
        );
        assert_eq!(params.len(), 2);
    }

    #[test]
    fn columns_and_conditions() {
        let mut builder = SqlQueryBuilder::new("table");

        builder.set_return(&["col_1"]);

        let val_1: i32 = 10;
        let val_2: i32 = 35;
        let val_3: i32 = 150;
        builder.add_condition("col_1", PostgresCmp::Less, &val_1);
        builder.add_condition("col_2", PostgresCmp::Equal, &val_2);
        builder.add_condition("col_3", PostgresCmp::Greater, &val_3);

        let (statement, params) = builder.build_select();

        assert_eq!(
            statement.as_str(),
            "SELECT col_1 FROM table WHERE col_1 < $1 AND col_2 = $2 AND col_3 > $3"
        );
        assert_eq!(params.len(), 3);
    }

    #[test]
    fn inner_join() {
        let mut builder = SqlQueryBuilder::new("table");

        builder.add_join(Join::Inner, "table2", "col_1");

        let (statement, params) = builder.build_select();

        assert_eq!(
            statement.as_str(),
            "SELECT * FROM table INNER JOIN table2 USING (col_1)"
        );
        assert_eq!(params.len(), 0,);
    }

    #[test]
    fn left_join() {
        let mut builder = SqlQueryBuilder::new("table");

        builder.add_join(Join::Left, "table2", "col_1");

        let (statement, params) = builder.build_select();

        assert_eq!(
            statement.as_str(),
            "SELECT * FROM table LEFT JOIN table2 USING (col_1)"
        );
        assert_eq!(params.len(), 0,);
    }

    #[test]
    fn right_join() {
        let mut builder = SqlQueryBuilder::new("table");

        builder.add_join(Join::Right, "table2", "col_1");

        let (statement, params) = builder.build_select();

        assert_eq!(
            statement.as_str(),
            "SELECT * FROM table RIGHT JOIN table2 USING (col_1)"
        );
        assert_eq!(params.len(), 0,);
    }

    #[test]
    fn full_join() {
        let mut builder = SqlQueryBuilder::new("table");

        builder.add_join(Join::Full, "table2", "col_1");

        let (statement, params) = builder.build_select();

        assert_eq!(
            statement.as_str(),
            "SELECT * FROM table FULL JOIN table2 USING (col_1)"
        );
        assert_eq!(params.len(), 0,);
    }

    #[test]
    fn multi_join() {
        let mut builder = SqlQueryBuilder::new("table");

        builder.add_join(Join::Inner, "table2", "col_1");
        builder.add_join(Join::Inner, "table3", "col_2");

        let (statement, params) = builder.build_select();

        assert_eq!(
            statement.as_str(),
            "SELECT * FROM table INNER JOIN table2 USING (col_1) INNER JOIN table3 USING (col_2)"
        );
        assert_eq!(params.len(), 0,);
    }

    #[test]
    fn group_by() {
        let mut builder = SqlQueryBuilder::new("table");

        builder.set_group_by(&["col_1"]);

        let (statement, params) = builder.build_select();

        assert_eq!(statement.as_str(), "SELECT * FROM table GROUP BY col_1",);
        assert_eq!(params.len(), 0);
    }

    #[test]
    fn having() {
        let mut builder = SqlQueryBuilder::new("table");

        let val_1: i32 = 10;
        builder.set_having("COUNT(col_1)", PostgresCmp::Equal, &val_1);

        let (statement, params) = builder.build_select();

        assert_eq!(
            statement.as_str(),
            "SELECT * FROM table HAVING COUNT(col_1) = $1",
        );
        assert_eq!(params.len(), 1);
    }
}

#[cfg(test)]
mod insert_builder {
    use super::SqlQueryBuilder;

    #[test]
    #[should_panic]
    fn empty() {
        let builder = SqlQueryBuilder::new("table");

        builder.build_insert();
    }

    #[test]
    fn one_column() {
        let mut builder = SqlQueryBuilder::new("table");

        let col_1 = String::from("Sample data");
        builder.add_column("col_1", &col_1);

        let (statement, params) = builder.build_insert();

        assert_eq!(statement.as_str(), "INSERT INTO table (col_1) VALUES ($1)");
        assert_eq!(params.len(), 1);
    }

    #[test]
    fn many_columns() {
        let mut builder = SqlQueryBuilder::new("table");

        let col_1 = uuid::Uuid::new_v4();
        let col_2 = String::from("Sample Data");
        let col_3 = chrono::Local::now().date_naive();
        builder.add_column("col_1", &col_1);
        builder.add_column("col_2", &col_2);
        builder.add_column("col_3", &col_3);

        let (statement, params) = builder.build_insert();

        assert_eq!(
            statement.as_str(),
            "INSERT INTO table (col_1, col_2, col_3) VALUES ($1, $2, $3)"
        );
        assert_eq!(params.len(), 3);
    }

    #[test]
    fn return_one_column() {
        let mut builder = SqlQueryBuilder::new("table");

        let col_1 = String::from("Sample data");
        builder.add_column("col_1", &col_1);

        builder.set_return(&["col_1"]);

        let (statement, params) = builder.build_insert();

        assert_eq!(
            statement.as_str(),
            "INSERT INTO table (col_1) VALUES ($1) RETURNING col_1"
        );
        assert_eq!(params.len(), 1);
    }

    #[test]
    fn return_many_columns() {
        let mut builder = SqlQueryBuilder::new("table");

        let col_1 = uuid::Uuid::new_v4();
        let col_2 = String::from("Sample Data");
        let col_3 = chrono::Local::now().date_naive();
        builder.add_column("col_1", &col_1);
        builder.add_column("col_2", &col_2);
        builder.add_column("col_3", &col_3);

        builder.set_return(&["col_1", "col_2"]);

        let (statement, params) = builder.build_insert();

        assert_eq!(
            statement.as_str(),
            "INSERT INTO table (col_1, col_2, col_3) VALUES ($1, $2, $3) RETURNING col_1, col_2"
        );
        assert_eq!(params.len(), 3);
    }

    #[test]
    fn return_all_columns() {
        let mut builder = SqlQueryBuilder::new("table");

        let col_1 = uuid::Uuid::new_v4();
        let col_2 = String::from("Sample Data");
        let col_3 = chrono::Local::now().date_naive();
        builder.add_column("col_1", &col_1);
        builder.add_column("col_2", &col_2);
        builder.add_column("col_3", &col_3);

        builder.set_return_all();

        let (statement, params) = builder.build_insert();

        assert_eq!(
            statement.as_str(),
            "INSERT INTO table (col_1, col_2, col_3) VALUES ($1, $2, $3) RETURNING *"
        );
        assert_eq!(params.len(), 3);
    }
}

#[cfg(test)]
mod update_builder {
    use crate::util::NULL;

    use super::{PostgresCmp, SqlQueryBuilder};

    #[test]
    #[should_panic]
    fn empty() {
        let builder = SqlQueryBuilder::new("table");

        builder.build_update();
    }

    #[test]
    fn one_column() {
        let mut builder = SqlQueryBuilder::new("table");

        let col_1 = String::from("Sample Data");
        builder.add_column("col_1", &col_1);

        let (statement, params) = builder.build_update();

        assert_eq!(statement.as_str(), "UPDATE table SET col_1=$1");
        assert_eq!(params.len(), 1);
    }

    #[test]
    fn many_columns() {
        let mut builder = SqlQueryBuilder::new("table");

        let col_1 = uuid::Uuid::new_v4();
        let col_2 = String::from("Sample Data");
        let col_3 = chrono::Local::now().date_naive();
        builder.add_column("col_1", &col_1);
        builder.add_column("col_2", &col_2);
        builder.add_column("col_3", &col_3);

        let (statement, params) = builder.build_update();

        assert_eq!(
            statement.as_str(),
            "UPDATE table SET col_1=$1, col_2=$2, col_3=$3"
        );
        assert_eq!(params.len(), 3);
    }

    #[test]
    fn one_condition() {
        let mut builder = SqlQueryBuilder::new("table");

        let col_1 = String::from("Sample Data");
        builder.add_column("col_1", &col_1);

        let val_1: i32 = 150;
        builder.add_condition("col_2", PostgresCmp::Less, &val_1);

        let (statement, params) = builder.build_update();

        assert_eq!(
            statement.as_str(),
            "UPDATE table SET col_1=$1 WHERE col_2 < $2"
        );
        assert_eq!(params.len(), 2)
    }

    #[test]
    fn many_conditions() {
        let mut builder = SqlQueryBuilder::new("table");

        let col_1 = String::from("Sample Data");
        builder.add_column("col_1", &col_1);

        let val_1: i32 = 150;
        let val_2: i32 = 14;
        builder.add_condition("col_2", PostgresCmp::Less, &val_1);
        builder.add_condition("col_3", PostgresCmp::Equal, &val_2);
        builder.add_condition("col_4", PostgresCmp::NotNull, &NULL);

        let (statement, params) = builder.build_update();

        assert_eq!(
            statement.as_str(),
            "UPDATE table SET col_1=$1 WHERE col_2 < $2 AND col_3 = $3 AND col_4 NOT NULL"
        );
        assert_eq!(params.len(), 3);
    }

    #[test]
    fn columns_and_conditions() {
        let mut builder = SqlQueryBuilder::new("table");

        let col_1 = uuid::Uuid::new_v4();
        let col_2 = String::from("Sample Data");
        let col_3 = chrono::Local::now().date_naive();
        builder.add_column("col_1", &col_1);
        builder.add_column("col_2", &col_2);
        builder.add_column("col_3", &col_3);

        let val_1: i32 = 150;
        let val_2: i32 = 14;
        builder.add_condition("col_2", PostgresCmp::Less, &val_1);
        builder.add_condition("col_3", PostgresCmp::Equal, &val_2);

        let (statement, params) = builder.build_update();

        assert_eq!(
            statement.as_str(),
            "UPDATE table SET col_1=$1, col_2=$2, col_3=$3 WHERE col_2 < $4 AND col_3 = $5"
        );
        assert_eq!(params.len(), 5);
    }

    #[test]
    fn return_one_column() {
        let mut builder = SqlQueryBuilder::new("table");

        let col_1 = String::from("Sample Data");
        builder.add_column("col_1", &col_1);

        builder.set_return(&["col_1"]);

        let (statement, params) = builder.build_update();

        assert_eq!(
            statement.as_str(),
            "UPDATE table SET col_1=$1 RETURNING col_1"
        );
        assert_eq!(params.len(), 1);
    }

    #[test]
    fn return_many_columns() {
        let mut builder = SqlQueryBuilder::new("table");

        let col_1 = String::from("Sample Data");
        builder.add_column("col_1", &col_1);

        builder.set_return(&["col_1", "col_2", "col_3"]);

        let (statement, params) = builder.build_update();

        assert_eq!(
            statement.as_str(),
            "UPDATE table SET col_1=$1 RETURNING col_1, col_2, col_3"
        );
        assert_eq!(params.len(), 1);
    }

    #[test]
    fn return_all_columns() {
        let mut builder = SqlQueryBuilder::new("table");

        let col_1 = String::from("Sample Data");
        builder.add_column("col_1", &col_1);

        let val_1: i32 = 150;
        builder.add_condition("col_2", PostgresCmp::Less, &val_1);

        builder.set_return_all();

        let (statement, params) = builder.build_update();

        assert_eq!(
            statement.as_str(),
            "UPDATE table SET col_1=$1 WHERE col_2 < $2 RETURNING *"
        );
        assert_eq!(params.len(), 2);
    }
}

#[cfg(test)]
mod delete_builder {
    use super::{PostgresCmp, SqlQueryBuilder};

    #[test]
    fn empty() {
        let builder = SqlQueryBuilder::new("table");

        let (statement, params) = builder.build_delete();

        assert_eq!(statement.as_str(), "DELETE FROM table");
        assert_eq!(params.len(), 0);
    }

    #[test]
    fn one_condition() {
        let mut builder = SqlQueryBuilder::new("table");

        let val_1: i32 = 150;
        builder.add_condition("col_2", PostgresCmp::Less, &val_1);

        let (statement, params) = builder.build_delete();

        assert_eq!(statement.as_str(), "DELETE FROM table WHERE col_2 < $1");
        assert_eq!(params.len(), 1);
    }

    #[test]
    fn many_conditions() {
        let mut builder = SqlQueryBuilder::new("table");

        let val_1: i32 = 150;
        let val_2: i32 = 18;
        builder.add_condition("col_2", PostgresCmp::Less, &val_1);
        builder.add_condition("col_3", PostgresCmp::Equal, &val_2);

        let (statement, params) = builder.build_delete();

        assert_eq!(
            statement.as_str(),
            "DELETE FROM table WHERE col_2 < $1 AND col_3 = $2"
        );
        assert_eq!(params.len(), 2);
    }

    #[test]
    fn return_one_column() {
        let mut builder = SqlQueryBuilder::new("table");

        builder.set_return(&["col_1"]);

        let (statement, params) = builder.build_delete();

        assert_eq!(statement.as_str(), "DELETE FROM table RETURNING col_1");
        assert_eq!(params.len(), 0);
    }

    #[test]
    fn return_many_columns() {
        let mut builder = SqlQueryBuilder::new("table");

        builder.set_return(&["col_1", "col_2"]);

        let (statement, params) = builder.build_delete();

        assert_eq!(
            statement.as_str(),
            "DELETE FROM table RETURNING col_1, col_2"
        );
        assert_eq!(params.len(), 0);
    }

    #[test]
    fn return_all_columns() {
        let mut builder = SqlQueryBuilder::new("table");

        builder.set_return_all();

        let (statement, params) = builder.build_delete();

        assert_eq!(statement.as_str(), "DELETE FROM table RETURNING *");
        assert_eq!(params.len(), 0);
    }
}
