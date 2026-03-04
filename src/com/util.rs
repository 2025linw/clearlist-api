use std::borrow::Cow;

use chrono::NaiveDate;
use serde::{Deserialize, Deserializer, de::Error};

pub fn deserialize_daterange<'de, D>(deserialize: D) -> Result<[NaiveDate; 2], D::Error>
where
    D: Deserializer<'de>,
{
    let s: Cow<'_, str> = Deserialize::deserialize(deserialize)?;

    if s.trim().is_empty() {
        return Err(D::Error::custom("date range cannot be empty"));
    }

    let mut parts = s.split('/');

    let start = parts
        .next()
        .ok_or_else(|| D::Error::custom("missing start date"))?
        .parse::<NaiveDate>()
        .map_err(D::Error::custom)?;

    let end = parts
        .next()
        .ok_or_else(|| D::Error::custom("missing end date"))?
        .parse::<NaiveDate>()
        .map_err(D::Error::custom)?;

    if parts.next().is_some() {
        return Err(D::Error::custom("too many dates in range"));
    }

    Ok([start, end])
}
