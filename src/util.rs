use tokio_postgres::types::ToSql;

use crate::com::model::db::SQLCmp;

#[derive(Debug)]
pub struct SQLBuilder {
    table: String,
    returning: Vec<String>,
    conditions: Vec<String>,
    owned: Vec<Box<dyn ToSql + Sync + Send>>,
    limit: Option<i64>,
    offset: Option<i64>,
}

impl SQLBuilder {
    pub fn new(table: &str) -> Self {
        Self {
            table: table.to_string(),
            returning: Vec::new(),
            conditions: Vec::new(),
            owned: Vec::new(),
            limit: None,
            offset: None,
        }
    }

    pub fn set_returning_str(&mut self, returning: &[&str]) {
        self.returning = returning.iter().map(|s| s.to_string()).collect();
    }

    pub fn add_condition<T>(&mut self, column: &str, cmp: SQLCmp, param: T)
    where
        T: ToSql + Sync + Send + 'static,
    {
        self.owned.push(Box::new(param));
        self.conditions
            .push(format!("{} {} ${}", column, cmp, self.owned.len()));
    }

    pub fn set_limit(&mut self, limit: i64) {
        self.limit = Some(limit);
    }

    pub fn set_offset(&mut self, offset: i64) {
        self.offset = Some(offset);
    }

    pub fn params(&self) -> Vec<&(dyn ToSql + Sync)> {
        self.owned
            .iter()
            .map(|b| b.as_ref() as &(dyn ToSql + Sync))
            .collect()
    }

    pub fn select_query(&self) -> String {
        let returning_clause = if self.returning.is_empty() {
            "*".to_string()
        } else {
            self.returning.join(", ")
        };
        let where_clause = if self.conditions.is_empty() {
            "".to_string()
        } else {
            format!("WHERE {}", self.conditions.join(" AND "))
        };
        let limit_clause = if let Some(limit) = self.limit {
            format!("LIMIT {}", limit)
        } else {
            "".to_string()
        };
        let offset_clause = if let Some(offset) = self.offset {
            format!("OFFSET {}", offset)
        } else {
            "".to_string()
        };

        format!(
            "SELECT {}
            FROM {}
            {}
            {}
            {}",
            returning_clause, self.table, where_clause, limit_clause, offset_clause,
        )
    }
}
