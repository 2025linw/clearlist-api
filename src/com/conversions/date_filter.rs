//! # Date Filter Conversions
//!
//! This module contains the conversion between route- and database-level date filter types

use crate::{
    db::{
        ApplicationError, Error,
        filters::{DateBound, DateFilter as DateFilterDB},
    },
    routes::models::{BracketInterval, DateFilter as DateFilterQuery},
};

impl TryFrom<DateFilterQuery> for DateFilterDB {
    type Error = Error;

    fn try_from(value: DateFilterQuery) -> Result<Self, Error> {
        match value {
            DateFilterQuery::Has(bool) => Ok(Self::Exists(bool)),
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
                interval => {
                    let ne = interval.ne.is_some()
                        && (interval.lt.is_some()
                            || interval.lte.is_some()
                            || interval.gt.is_some()
                            || interval.gte.is_some());
                    let greater = interval.gt.is_some() && interval.gte.is_some();
                    let less = interval.lt.is_some() && interval.lte.is_some();

                    if ne {
                        Err(Error::Application(ApplicationError::InvalidDateRange(
                            "unsupported combination of operators. use either a range ('<', '<=', '>', '>=') or exclusions ('!='), but not both".to_string()
                        )))
                    } else if greater && less {
                        Err(Error::Application(ApplicationError::InvalidDateRange(
                            "use only one of '<' or '<=' and one of '>' or '>='".to_string(),
                        )))
                    } else if less {
                        Err(Error::Application(ApplicationError::InvalidDateRange(
                            "use only one of '<' or '<='".to_string(),
                        )))
                    } else {
                        Err(Error::Application(ApplicationError::InvalidDateRange(
                            "use only one of '>' or '>='".to_string(),
                        )))
                    }
                }
            },
            DateFilterQuery::ISO8601Interval([start_date, end_date]) => Ok(DateFilterDB::Range(
                DateBound::Inclusive(start_date),
                DateBound::Inclusive(end_date),
            )),
        }
    }
}

#[cfg(test)]
mod date_filter_db {
    use chrono::{Duration, Local};

    use crate::{
        db::{ApplicationError, Error},
        routes::models::BracketInterval,
    };

    use super::{DateBound, DateFilterDB, DateFilterQuery};

    #[test]
    fn exact_test() {
        let today = Local::now().date_naive();

        let query = DateFilterQuery::Exact(today);

        assert_eq!(
            DateFilterDB::try_from(query).unwrap(),
            DateFilterDB::On(today)
        )
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
            DateFilterDB::try_from(query).unwrap(),
            DateFilterDB::NotOn(today)
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
            DateFilterDB::try_from(query).unwrap(),
            DateFilterDB::StartRange(DateBound::Exclusive(today))
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
            DateFilterDB::try_from(query).unwrap(),
            DateFilterDB::StartRange(DateBound::Inclusive(today))
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
            DateFilterDB::try_from(query).unwrap(),
            DateFilterDB::EndRange(DateBound::Exclusive(today))
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
            DateFilterDB::try_from(query).unwrap(),
            DateFilterDB::EndRange(DateBound::Inclusive(today))
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
            DateFilterDB::try_from(query).unwrap(),
            DateFilterDB::Range(
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
            DateFilterDB::try_from(query).unwrap(),
            DateFilterDB::Range(
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
            DateFilterDB::try_from(query).unwrap(),
            DateFilterDB::Range(
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
            DateFilterDB::try_from(query).unwrap(),
            DateFilterDB::Range(
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
        let res = DateFilterDB::try_from(query);
        assert!(res.is_err(), "'ne' should not be used with 'gt'");
        assert!(matches!(
            res,
            Err(Error::Application(ApplicationError::InvalidDateRange(_)))
        ));

        let query = DateFilterQuery::BracketInterval(BracketInterval {
            ne: Some(today),
            gt: None,
            gte: Some(today),
            lt: None,
            lte: None,
        });
        let res = DateFilterDB::try_from(query);
        assert!(res.is_err(), "'ne' should not be used with 'gte'");
        assert!(matches!(
            res,
            Err(Error::Application(ApplicationError::InvalidDateRange(_)))
        ));

        let query = DateFilterQuery::BracketInterval(BracketInterval {
            ne: Some(today),
            gt: None,
            gte: None,
            lt: Some(today),
            lte: None,
        });
        let res = DateFilterDB::try_from(query);
        assert!(res.is_err(), "'ne' should not be used with 'lt'");
        assert!(matches!(
            res,
            Err(Error::Application(ApplicationError::InvalidDateRange(_)))
        ));

        let query = DateFilterQuery::BracketInterval(BracketInterval {
            ne: Some(today),
            gt: None,
            gte: None,
            lt: None,
            lte: Some(today),
        });
        let res = DateFilterDB::try_from(query);
        assert!(res.is_err(), "'ne' should not be used with 'lte'");
        assert!(matches!(
            res,
            Err(Error::Application(ApplicationError::InvalidDateRange(_)))
        ));

        let query = DateFilterQuery::BracketInterval(BracketInterval {
            ne: Some(today),
            gt: None,
            gte: None,
            lt: None,
            lte: Some(today),
        });
        let res = DateFilterDB::try_from(query);
        assert!(res.is_err(), "'ne' should not be used with 'lte'");
        assert!(matches!(
            res,
            Err(Error::Application(ApplicationError::InvalidDateRange(_)))
        ));

        let query = DateFilterQuery::BracketInterval(BracketInterval {
            ne: Some(today),
            gt: Some(today),
            gte: None,
            lt: Some(today),
            lte: None,
        });
        let res = DateFilterDB::try_from(query);
        assert!(res.is_err(), "'ne' should not be used with 'gt' and 'lt");
        assert!(matches!(
            res,
            Err(Error::Application(ApplicationError::InvalidDateRange(_)))
        ));

        let query = DateFilterQuery::BracketInterval(BracketInterval {
            ne: Some(today),
            gt: Some(today),
            gte: None,
            lt: None,
            lte: Some(today),
        });
        let res = DateFilterDB::try_from(query);
        assert!(res.is_err(), "'ne' should not be used with 'gt' and 'lte");
        assert!(matches!(
            res,
            Err(Error::Application(ApplicationError::InvalidDateRange(_)))
        ));

        let query = DateFilterQuery::BracketInterval(BracketInterval {
            ne: Some(today),
            gt: None,
            gte: Some(today),
            lt: Some(today),
            lte: None,
        });
        let res = DateFilterDB::try_from(query);
        assert!(res.is_err(), "'ne' should not be used with 'gte' and 'lt");
        assert!(matches!(
            res,
            Err(Error::Application(ApplicationError::InvalidDateRange(_)))
        ));

        let query = DateFilterQuery::BracketInterval(BracketInterval {
            ne: Some(today),
            gt: None,
            gte: Some(today),
            lt: None,
            lte: Some(today),
        });
        let res = DateFilterDB::try_from(query);
        assert!(res.is_err(), "'ne' should not be used with 'gte' and 'lte");
        assert!(matches!(
            res,
            Err(Error::Application(ApplicationError::InvalidDateRange(_)))
        ));
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
        let res = DateFilterDB::try_from(query);
        assert!(res.is_err(), "can not convert empty bracket interval");
        assert!(matches!(
            res,
            Err(Error::Application(ApplicationError::InvalidDateRange(_)))
        ));
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
        let res = DateFilterDB::try_from(query);
        assert!(
            res.is_err(),
            "'gt' and 'gte' must not be specified together"
        );
        assert!(matches!(
            res,
            Err(Error::Application(ApplicationError::InvalidDateRange(_)))
        ));

        let query = DateFilterQuery::BracketInterval(BracketInterval {
            ne: None,
            gt: None,
            gte: None,
            lt: Some(today),
            lte: Some(today),
        });
        let res = DateFilterDB::try_from(query);
        assert!(
            res.is_err(),
            "'gt' and 'gte' must not be specified together"
        );
        assert!(matches!(
            res,
            Err(Error::Application(ApplicationError::InvalidDateRange(_)))
        ));

        let query = DateFilterQuery::BracketInterval(BracketInterval {
            ne: None,
            gt: Some(today),
            gte: Some(today),
            lt: Some(today),
            lte: None,
        });
        let res = DateFilterDB::try_from(query);
        assert!(
            res.is_err(),
            "'gt', 'gte', 'lt' must not be specified together"
        );
        assert!(matches!(
            res,
            Err(Error::Application(ApplicationError::InvalidDateRange(_)))
        ));

        let query = DateFilterQuery::BracketInterval(BracketInterval {
            ne: None,
            gt: Some(today),
            gte: Some(today),
            lt: None,
            lte: Some(today),
        });
        let res = DateFilterDB::try_from(query);
        assert!(
            res.is_err(),
            "'gt', 'gte', 'lte' must not be specified together"
        );
        assert!(matches!(
            res,
            Err(Error::Application(ApplicationError::InvalidDateRange(_)))
        ));

        let query = DateFilterQuery::BracketInterval(BracketInterval {
            ne: None,
            gt: Some(today),
            gte: None,
            lt: Some(today),
            lte: Some(today),
        });
        let res = DateFilterDB::try_from(query);
        assert!(
            res.is_err(),
            "'gt', 'lt', 'lte' must not be specified together"
        );
        assert!(matches!(
            res,
            Err(Error::Application(ApplicationError::InvalidDateRange(_)))
        ));

        let query = DateFilterQuery::BracketInterval(BracketInterval {
            ne: None,
            gt: None,
            gte: Some(today),
            lt: Some(today),
            lte: Some(today),
        });
        let res = DateFilterDB::try_from(query);
        assert!(
            res.is_err(),
            "'gte', 'lt', 'lte' must not be specified together"
        );
        assert!(matches!(
            res,
            Err(Error::Application(ApplicationError::InvalidDateRange(_)))
        ));
    }

    #[test]
    fn iso8601_interval() {
        let today = Local::now().date_naive();
        let today_1week = Local::now().date_naive() + Duration::weeks(1);

        let query = DateFilterQuery::ISO8601Interval([today, today_1week]);

        assert_eq!(
            DateFilterDB::try_from(query).unwrap(),
            DateFilterDB::Range(
                DateBound::Inclusive(today),
                DateBound::Inclusive(today_1week)
            )
        )
    }
}
