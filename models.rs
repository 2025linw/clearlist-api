pub mod area;
pub mod auth;
pub mod jwt;
pub mod project;
pub mod tag;
pub mod task;
pub mod user;

use bytes::BytesMut;
use serde::Deserialize;
use tokio_postgres::types::{IsNull, ToSql, Type, to_sql_checked};

use crate::error::Error;
use crate::util::{PostgresCmp, ToPostgresCmp};

/// Paging options for querying
#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct FilterOptions {
    pub page: Option<usize>,
    pub limit: Option<usize>,
}

impl Default for FilterOptions {
    fn default() -> Self {
        Self {
            page: Some(1),
            limit: Some(25),
        }
    }
}

/// Trait to convert from Database Model to Response Model
pub trait ToResponse {
    type Response;

    /// Converts type to Response
    fn to_response(&self) -> Self::Response;
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

/// Update request body elements
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", tag = "op", content = "value")]
pub enum UpdateMethod<T: ToSql> {
    #[default]
    NoOp,
    Remove,
    Set(T),
}

impl<T: ToSql> UpdateMethod<T> {
    pub fn is_noop(&self) -> bool {
        matches!(self, Self::NoOp)
    }
}

impl<T: ToSql> ToSql for UpdateMethod<T> {
    fn to_sql(
        &self,
        ty: &Type,
        out: &mut BytesMut,
    ) -> Result<IsNull, Box<dyn std::error::Error + Sync + Send>>
    where
        Self: Sized,
    {
        match *self {
            Self::Remove => Ok(IsNull::Yes),
            Self::Set(ref value) => value.to_sql(ty, out),
            Self::NoOp => Err(Box::new(Error::Internal(String::from(
                "UpdateMethod::NoOp can not be converted into SQL",
            )))),
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

/// Query request body elements
#[derive(Debug, Deserialize)]
// #[derive(Debug, Default, Deserialize)]
// TODO: add NoOp
#[serde(untagged)]
pub enum QueryMethod<T: ToSql> {
    // #[default]
    // NoOp,
    NotNull(bool),
    Match(T),
    Compare(T, Compare),
}

impl<T> ToSql for QueryMethod<T>
where
    T: ToSql,
{
    fn to_sql(
        &self,
        ty: &Type,
        out: &mut BytesMut,
    ) -> Result<IsNull, Box<dyn std::error::Error + Sync + Send>>
    where
        Self: Sized,
    {
        match *self {
            Self::Match(ref o) | Self::Compare(ref o, _) => o.to_sql(ty, out),
            _ => Err(Box::new(Error::Internal(String::from(
                "QueryMethod::NotNull can not be converted into SQL",
            )))),
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
