pub mod area;
pub mod auth;
pub mod project;
pub mod tag;
pub mod task;

use std::{error::Error, fmt::Debug};

use bytes::BytesMut;
use serde::Deserialize;
use tokio_postgres::types::{IsNull, ToSql, Type, to_sql_checked};

use crate::util::{PostgresCmp, ToPostgresCmp};

#[derive(Debug, Deserialize)]
pub struct FilterOptions {
    pub page: Option<usize>,
    pub limit: Option<usize>,
}

impl Default for FilterOptions {
    fn default() -> Self {
        Self {
            page: Some(0),
            limit: Some(25),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum UpdateMethod<T>
where
    T: ToSql,
{
    Remove(bool),
    Change(T),
}

impl<T: ToSql> ToSql for UpdateMethod<T> {
    fn to_sql(&self, ty: &Type, out: &mut BytesMut) -> Result<IsNull, Box<dyn Error + Sync + Send>>
    where
        Self: Sized,
    {
        match *self {
            Self::Remove(true) => Ok(IsNull::Yes),
            Self::Change(ref o) => o.to_sql(ty, out),
            Self::Remove(false) => Err(Box::from("no-op")),
        }
    }

    fn accepts(ty: &Type) -> bool
    where
        Self: Sized,
    {
        <T as ToSql>::accepts(ty)
    }

    to_sql_checked!();
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub enum Compare {
    Equal,
    NotEqual,
    Less,
    LessEq,
    Greater,
    GreaterEq,
}

impl ToPostgresCmp for Compare {
    fn to_postgres_cmp(&self) -> PostgresCmp {
        match self {
            Compare::Equal => PostgresCmp::Equal,
            Compare::NotEqual => PostgresCmp::NotEqual,
            Compare::Less => PostgresCmp::Less,
            Compare::LessEq => PostgresCmp::LessEq,
            Compare::Greater => PostgresCmp::Greater,
            Compare::GreaterEq => PostgresCmp::GreaterEq,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum QueryMethod<T: ToSql> {
    NotNull(bool),
    Match(T),
    Compare(T, Compare),
}

impl<T> ToSql for QueryMethod<T>
where
    T: ToSql,
{
    fn to_sql(&self, ty: &Type, out: &mut BytesMut) -> Result<IsNull, Box<dyn Error + Sync + Send>>
    where
        Self: Sized,
    {
        match *self {
            Self::Match(ref o) | Self::Compare(ref o, _) => o.to_sql(ty, out),
            _ => Err(Box::from("no-op")),
        }
    }

    fn accepts(ty: &Type) -> bool
    where
        Self: Sized,
    {
        <T as ToSql>::accepts(ty)
    }

    to_sql_checked!();
}
