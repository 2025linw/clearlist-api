use chrono::NaiveDate;

use crate::com::{
    error::Error,
    model::query::{BracketInterval, DateFilter as DateFilterQuery},
};

// TODO: add Exists and NotExists
pub enum SQLCmp {
    Equal,
    NotEqual,
    LessThan,
    LessThanEqual,
    GreaterThan,
    GreaterThanEqual,
}

impl std::fmt::Display for SQLCmp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SQLCmp::Equal => write!(f, "="),
            SQLCmp::NotEqual => write!(f, "<>"),
            SQLCmp::LessThan => write!(f, "<"),
            SQLCmp::LessThanEqual => write!(f, "<="),
            SQLCmp::GreaterThan => write!(f, ">"),
            SQLCmp::GreaterThanEqual => write!(f, ">="),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum DateBound {
    Exclusive(NaiveDate),
    Inclusive(NaiveDate),
}

#[derive(Debug, PartialEq)]
pub enum DateFilter {
    On(NaiveDate),
    NotOn(NaiveDate),
    StartRange(DateBound),
    EndRange(DateBound),
    Range(DateBound, DateBound),
}

impl DateFilter {
    pub fn into_sql(self) -> Vec<(SQLCmp, NaiveDate)> {
        match self {
            DateFilter::On(date) => vec![(SQLCmp::Equal, date)],
            DateFilter::NotOn(date) => vec![(SQLCmp::NotEqual, date)],
            DateFilter::StartRange(bound) => match bound {
                DateBound::Exclusive(start_date) => vec![(SQLCmp::GreaterThan, start_date)],
                DateBound::Inclusive(start_date) => vec![(SQLCmp::GreaterThanEqual, start_date)],
            },
            DateFilter::EndRange(bound) => match bound {
                DateBound::Exclusive(end_date) => vec![(SQLCmp::LessThan, end_date)],
                DateBound::Inclusive(end_date) => vec![(SQLCmp::LessThanEqual, end_date)],
            },
            DateFilter::Range(start, end) => match (start, end) {
                (DateBound::Exclusive(start_date), DateBound::Exclusive(end_date)) => vec![
                    (SQLCmp::GreaterThan, start_date),
                    (SQLCmp::LessThan, end_date),
                ],
                (DateBound::Exclusive(start_date), DateBound::Inclusive(end_date)) => vec![
                    (SQLCmp::GreaterThan, start_date),
                    (SQLCmp::LessThanEqual, end_date),
                ],
                (DateBound::Inclusive(start_date), DateBound::Exclusive(end_date)) => vec![
                    (SQLCmp::GreaterThanEqual, start_date),
                    (SQLCmp::LessThan, end_date),
                ],
                (DateBound::Inclusive(start_date), DateBound::Inclusive(end_date)) => vec![
                    (SQLCmp::GreaterThanEqual, start_date),
                    (SQLCmp::LessThanEqual, end_date),
                ],
            },
        }
    }
}

impl TryFrom<DateFilterQuery> for DateFilter {
    type Error = Error;

    fn try_from(value: DateFilterQuery) -> Result<Self, Error> {
        match value {
            DateFilterQuery::Exact(date) => Ok(Self::On(date)),
            DateFilterQuery::BracketInterval(interval) => match interval {
                BracketInterval {
                    ne: Some(date),
                    gt: None,
                    gte: None,
                    lt: None,
                    lte: None,
                } => Ok(Self::NotOn(date)),
                BracketInterval {
                    ne: None,
                    gt: None,
                    gte: None,
                    lt: Some(date),
                    lte: None,
                } => Ok(Self::EndRange(DateBound::Exclusive(date))),
                BracketInterval {
                    ne: None,
                    gt: None,
                    gte: None,
                    lt: None,
                    lte: Some(date),
                } => Ok(Self::EndRange(DateBound::Inclusive(date))),
                BracketInterval {
                    ne: None,
                    gt: Some(date),
                    gte: None,
                    lt: None,
                    lte: None,
                } => Ok(Self::StartRange(DateBound::Exclusive(date))),
                BracketInterval {
                    ne: None,
                    gt: None,
                    gte: Some(date),
                    lt: None,
                    lte: None,
                } => Ok(Self::StartRange(DateBound::Inclusive(date))),
                BracketInterval {
                    ne: None,
                    gt: Some(start_date),
                    gte: None,
                    lt: Some(end_date),
                    lte: None,
                } => Ok(Self::Range(
                    DateBound::Exclusive(start_date),
                    DateBound::Exclusive(end_date),
                )),
                BracketInterval {
                    ne: None,
                    gt: Some(start_date),
                    gte: None,
                    lt: None,
                    lte: Some(end_date),
                } => Ok(Self::Range(
                    DateBound::Exclusive(start_date),
                    DateBound::Inclusive(end_date),
                )),
                BracketInterval {
                    ne: None,
                    gt: None,
                    gte: Some(start_date),
                    lt: Some(end_date),
                    lte: None,
                } => Ok(Self::Range(
                    DateBound::Inclusive(start_date),
                    DateBound::Exclusive(end_date),
                )),
                BracketInterval {
                    ne: None,
                    gt: None,
                    gte: Some(start_date),
                    lt: None,
                    lte: Some(end_date),
                } => Ok(Self::Range(
                    DateBound::Inclusive(start_date),
                    DateBound::Inclusive(end_date),
                )),
                _ => Err(Error::DateRangeConversion(
                    "unable to convert from query filter to db filter".to_string(),
                )),
            },
            DateFilterQuery::ISO8601Interval([start_date, end_date]) => Ok(DateFilter::Range(
                DateBound::Inclusive(start_date),
                DateBound::Inclusive(end_date),
            )),
        }
    }
}

#[derive(Default)]
pub enum SortOrder {
    #[default]
    UpdatedDesc,
    UpdatedAsc,
    CreatedDesc,
    CreatedAsc,
}

#[cfg(test)]
mod date_filter_db {
    use chrono::{Duration, Local};

    use crate::com::model::query::{BracketInterval, DateFilter as DateFilterQuery};

    use super::{DateBound, DateFilter, Error};

    #[test]
    fn exact_test() {
        let today = Local::now().date_naive();

        let query = DateFilterQuery::Exact(today);

        assert_eq!(DateFilter::try_from(query).unwrap(), DateFilter::On(today))
    }

    #[test]
    fn bracket_ne() {
        let today = Local::now().date_naive();

        let query = DateFilterQuery::BracketInterval(BracketInterval {
            ne: Some(today),
            gt: None,
            gte: None,
            lt: None,
            lte: None,
        });

        assert_eq!(
            DateFilter::try_from(query).unwrap(),
            DateFilter::NotOn(today)
        )
    }

    #[test]
    fn bracket_start_exclusive() {
        let today = Local::now().date_naive();

        let query = DateFilterQuery::BracketInterval(BracketInterval {
            ne: None,
            gt: Some(today),
            gte: None,
            lt: None,
            lte: None,
        });

        assert_eq!(
            DateFilter::try_from(query).unwrap(),
            DateFilter::StartRange(DateBound::Exclusive(today))
        )
    }

    #[test]
    fn bracket_start_inclusive() {
        let today = Local::now().date_naive();

        let query = DateFilterQuery::BracketInterval(BracketInterval {
            ne: None,
            gt: None,
            gte: Some(today),
            lt: None,
            lte: None,
        });

        assert_eq!(
            DateFilter::try_from(query).unwrap(),
            DateFilter::StartRange(DateBound::Inclusive(today))
        )
    }

    #[test]
    fn bracket_end_exclusive() {
        let today = Local::now().date_naive();

        let query = DateFilterQuery::BracketInterval(BracketInterval {
            ne: None,
            gt: None,
            gte: None,
            lt: Some(today),
            lte: None,
        });

        assert_eq!(
            DateFilter::try_from(query).unwrap(),
            DateFilter::EndRange(DateBound::Exclusive(today))
        )
    }

    #[test]
    fn bracket_end_inclusive() {
        let today = Local::now().date_naive();

        let query = DateFilterQuery::BracketInterval(BracketInterval {
            ne: None,
            gt: None,
            gte: None,
            lt: None,
            lte: Some(today),
        });

        assert_eq!(
            DateFilter::try_from(query).unwrap(),
            DateFilter::EndRange(DateBound::Inclusive(today))
        )
    }

    #[test]
    fn bracket_range_start_excl_end_excl() {
        let today = Local::now().date_naive();
        let today_1week = Local::now().date_naive() + Duration::weeks(1);

        let query = DateFilterQuery::BracketInterval(BracketInterval {
            ne: None,
            gt: Some(today),
            gte: None,
            lt: Some(today_1week),
            lte: None,
        });

        assert_eq!(
            DateFilter::try_from(query).unwrap(),
            DateFilter::Range(
                DateBound::Exclusive(today),
                DateBound::Exclusive(today_1week)
            )
        )
    }

    #[test]
    fn bracket_range_start_excl_end_incl() {
        let today = Local::now().date_naive();
        let today_1week = Local::now().date_naive() + Duration::weeks(1);

        let query = DateFilterQuery::BracketInterval(BracketInterval {
            ne: None,
            gt: Some(today),
            gte: None,
            lt: None,
            lte: Some(today_1week),
        });

        assert_eq!(
            DateFilter::try_from(query).unwrap(),
            DateFilter::Range(
                DateBound::Exclusive(today),
                DateBound::Inclusive(today_1week)
            )
        )
    }

    #[test]
    fn bracket_range_start_incl_end_excl() {
        let today = Local::now().date_naive();
        let today_1week = Local::now().date_naive() + Duration::weeks(1);

        let query = DateFilterQuery::BracketInterval(BracketInterval {
            ne: None,
            gt: None,
            gte: Some(today),
            lt: Some(today_1week),
            lte: None,
        });

        assert_eq!(
            DateFilter::try_from(query).unwrap(),
            DateFilter::Range(
                DateBound::Inclusive(today),
                DateBound::Exclusive(today_1week)
            )
        )
    }

    #[test]
    fn bracket_range_start_incl_end_incl() {
        let today = Local::now().date_naive();
        let today_1week = Local::now().date_naive() + Duration::weeks(1);

        let query = DateFilterQuery::BracketInterval(BracketInterval {
            ne: None,
            gt: None,
            gte: Some(today),
            lt: None,
            lte: Some(today_1week),
        });

        assert_eq!(
            DateFilter::try_from(query).unwrap(),
            DateFilter::Range(
                DateBound::Inclusive(today),
                DateBound::Inclusive(today_1week)
            )
        )
    }

    #[test]
    fn bracket_range_ne_with_other_panic() {
        let today = Local::now().date_naive();

        let query = DateFilterQuery::BracketInterval(BracketInterval {
            ne: Some(today),
            gt: Some(today),
            gte: None,
            lt: None,
            lte: None,
        });
        let res = DateFilter::try_from(query);
        assert!(res.is_err(), "'ne' should not be used with 'gt'");
        assert!(matches!(res, Err(Error::DateRangeConversion(_))));

        let query = DateFilterQuery::BracketInterval(BracketInterval {
            ne: Some(today),
            gt: None,
            gte: Some(today),
            lt: None,
            lte: None,
        });
        let res = DateFilter::try_from(query);
        assert!(res.is_err(), "'ne' should not be used with 'gte'");
        assert!(matches!(res, Err(Error::DateRangeConversion(_))));

        let query = DateFilterQuery::BracketInterval(BracketInterval {
            ne: Some(today),
            gt: None,
            gte: None,
            lt: Some(today),
            lte: None,
        });
        let res = DateFilter::try_from(query);
        assert!(res.is_err(), "'ne' should not be used with 'lt'");
        assert!(matches!(res, Err(Error::DateRangeConversion(_))));

        let query = DateFilterQuery::BracketInterval(BracketInterval {
            ne: Some(today),
            gt: None,
            gte: None,
            lt: None,
            lte: Some(today),
        });
        let res = DateFilter::try_from(query);
        assert!(res.is_err(), "'ne' should not be used with 'lte'");
        assert!(matches!(res, Err(Error::DateRangeConversion(_))));

        let query = DateFilterQuery::BracketInterval(BracketInterval {
            ne: Some(today),
            gt: None,
            gte: None,
            lt: None,
            lte: Some(today),
        });
        let res = DateFilter::try_from(query);
        assert!(res.is_err(), "'ne' should not be used with 'lte'");
        assert!(matches!(res, Err(Error::DateRangeConversion(_))));

        let query = DateFilterQuery::BracketInterval(BracketInterval {
            ne: Some(today),
            gt: Some(today),
            gte: None,
            lt: Some(today),
            lte: None,
        });
        let res = DateFilter::try_from(query);
        assert!(res.is_err(), "'ne' should not be used with 'gt' and 'lt");
        assert!(matches!(res, Err(Error::DateRangeConversion(_))));

        let query = DateFilterQuery::BracketInterval(BracketInterval {
            ne: Some(today),
            gt: Some(today),
            gte: None,
            lt: None,
            lte: Some(today),
        });
        let res = DateFilter::try_from(query);
        assert!(res.is_err(), "'ne' should not be used with 'gt' and 'lte");
        assert!(matches!(res, Err(Error::DateRangeConversion(_))));

        let query = DateFilterQuery::BracketInterval(BracketInterval {
            ne: Some(today),
            gt: None,
            gte: Some(today),
            lt: Some(today),
            lte: None,
        });
        let res = DateFilter::try_from(query);
        assert!(res.is_err(), "'ne' should not be used with 'gte' and 'lt");
        assert!(matches!(res, Err(Error::DateRangeConversion(_))));

        let query = DateFilterQuery::BracketInterval(BracketInterval {
            ne: Some(today),
            gt: None,
            gte: Some(today),
            lt: None,
            lte: Some(today),
        });
        let res = DateFilter::try_from(query);
        assert!(res.is_err(), "'ne' should not be used with 'gte' and 'lte");
        assert!(matches!(res, Err(Error::DateRangeConversion(_))));
    }

    #[test]
    fn bracket_range_range_underspecification() {
        let query = DateFilterQuery::BracketInterval(BracketInterval {
            ne: None,
            gt: None,
            gte: None,
            lt: None,
            lte: None,
        });
        let res = DateFilter::try_from(query);
        assert!(res.is_err(), "can not convert empty bracket interval");
        assert!(matches!(res, Err(Error::DateRangeConversion(_))));
    }

    #[test]
    fn bracket_range_range_overspecification() {
        let today = Local::now().date_naive();

        let query = DateFilterQuery::BracketInterval(BracketInterval {
            ne: None,
            gt: Some(today),
            gte: Some(today),
            lt: None,
            lte: None,
        });
        let res = DateFilter::try_from(query);
        assert!(
            res.is_err(),
            "'gt' and 'gte' must not be specified together"
        );
        assert!(matches!(res, Err(Error::DateRangeConversion(_))));

        let query = DateFilterQuery::BracketInterval(BracketInterval {
            ne: None,
            gt: None,
            gte: None,
            lt: Some(today),
            lte: Some(today),
        });
        let res = DateFilter::try_from(query);
        assert!(
            res.is_err(),
            "'gt' and 'gte' must not be specified together"
        );
        assert!(matches!(res, Err(Error::DateRangeConversion(_))));

        let query = DateFilterQuery::BracketInterval(BracketInterval {
            ne: None,
            gt: Some(today),
            gte: Some(today),
            lt: Some(today),
            lte: None,
        });
        let res = DateFilter::try_from(query);
        assert!(
            res.is_err(),
            "'gt', 'gte', 'lt' must not be specified together"
        );
        assert!(matches!(res, Err(Error::DateRangeConversion(_))));

        let query = DateFilterQuery::BracketInterval(BracketInterval {
            ne: None,
            gt: Some(today),
            gte: Some(today),
            lt: None,
            lte: Some(today),
        });
        let res = DateFilter::try_from(query);
        assert!(
            res.is_err(),
            "'gt', 'gte', 'lte' must not be specified together"
        );
        assert!(matches!(res, Err(Error::DateRangeConversion(_))));

        let query = DateFilterQuery::BracketInterval(BracketInterval {
            ne: None,
            gt: Some(today),
            gte: None,
            lt: Some(today),
            lte: Some(today),
        });
        let res = DateFilter::try_from(query);
        assert!(
            res.is_err(),
            "'gt', 'lt', 'lte' must not be specified together"
        );
        assert!(matches!(res, Err(Error::DateRangeConversion(_))));

        let query = DateFilterQuery::BracketInterval(BracketInterval {
            ne: None,
            gt: None,
            gte: Some(today),
            lt: Some(today),
            lte: Some(today),
        });
        let res = DateFilter::try_from(query);
        assert!(
            res.is_err(),
            "'gte', 'lt', 'lte' must not be specified together"
        );
        assert!(matches!(res, Err(Error::DateRangeConversion(_))));
    }

    #[test]
    fn iso8601_interval() {
        let today = Local::now().date_naive();
        let today_1week = Local::now().date_naive() + Duration::weeks(1);

        let query = DateFilterQuery::ISO8601Interval([today, today_1week]);

        assert_eq!(
            DateFilter::try_from(query).unwrap(),
            DateFilter::Range(
                DateBound::Inclusive(today),
                DateBound::Inclusive(today_1week)
            )
        )
    }
}
