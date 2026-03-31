use chrono::Utc;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Start {
    On(chrono::NaiveDate),
    At(chrono::DateTime<Utc>),
}

impl Start {
    pub fn as_on(&self) -> Option<chrono::NaiveDate> {
        match self {
            Self::On(date) => Some(*date),
            Self::At(_) => None,
        }
    }

    pub fn as_at(&self) -> Option<chrono::DateTime<Utc>> {
        match self {
            Self::On(_) => None,
            Self::At(datetime) => Some(*datetime),
        }
    }
}
