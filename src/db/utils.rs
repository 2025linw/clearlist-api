use chrono::{NaiveDate, NaiveTime};
use tokio_postgres::types::ToSql;

#[derive(Debug)]
pub enum DateCmp<T> {
    On(T),
    Before(T),
    OnBefore(T),
    After(T),
    OnAfter(T),
}

impl<T> DateCmp<T> {
    pub fn to_sql(&self, column: &str, n: usize) -> String {
        match self {
            DateCmp::On(_) => format!("{column} = ${n}"),
            DateCmp::Before(_) => format!("{column} < ${n}"),
            DateCmp::OnBefore(_) => format!("{column} <= ${n}"),
            DateCmp::After(_) => format!("{column} > ${n}"),
            DateCmp::OnAfter(_) => format!("{column} >= ${n}"),
        }
    }
}

impl<T> ToSql for DateCmp<T>
where
    T: std::fmt::Debug,
{
    fn to_sql(
        &self,
        ty: &tokio_postgres::types::Type,
        out: &mut bytes::BytesMut,
    ) -> Result<tokio_postgres::types::IsNull, Box<dyn std::error::Error + Sync + Send>>
    where
        Self: Sized,
    {
        todo!()
    }

    fn accepts(ty: &tokio_postgres::types::Type) -> bool
    where
        Self: Sized,
    {
        todo!()
    }

    fn to_sql_checked(
        &self,
        ty: &tokio_postgres::types::Type,
        out: &mut bytes::BytesMut,
    ) -> Result<tokio_postgres::types::IsNull, Box<dyn std::error::Error + Sync + Send>> {
        todo!()
    }
}

pub struct DateTimeFilter {
    start_date: DateCmp<NaiveDate>,
    start_time: DateCmp<NaiveTime>,
    deadline: DateCmp<NaiveDate>,
}
