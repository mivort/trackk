use builtin_queries as bq;
use serde_derive::Deserialize;

use super::Config;
use crate::prelude::*;

/// Query config which defines filter expression and sorting columns.
#[derive(Deserialize, Default, Clone)]
#[cfg_attr(test, derive(Debug, PartialEq, Eq))]
pub struct QueryConfig {
    /// Sorting columns and directions.
    #[serde(default)]
    pub sorting: Box<str>,

    /// Section filter query.
    #[serde(default)]
    pub filter: Box<str>,

    /// Group datecalc query.
    #[serde(default)]
    pub group_by: Box<str>,

    /// Index to use when query is called.
    #[serde(default)]
    pub index: IndexType,
}

/// Reference to query data.
#[derive(Default)]
pub struct QueryData<'a> {
    pub sorting: &'a str,
    pub filter: &'a str,
    pub group_by: &'a str,
    pub index: IndexType,
}

#[derive(Deserialize, Default, Clone, Copy)]
#[cfg_attr(test, derive(Debug, PartialEq, Eq))]
pub enum IndexType {
    #[default]
    #[serde(rename = "active")]
    Active,
    #[serde(rename = "recent")]
    Recent,
    #[serde(rename = "all")]
    All,
}

pub mod builtin_queries {
    pub const BACKLOG: &str = "backlog";
    pub const UPCOMING: &str = "upcoming";
    pub const CURRENT: &str = "current";
    pub const DUE_TODAY: &str = "due_today";
    pub const STARTED: &str = "started";
    pub const DONE_TODAY: &str = "done_today";
    pub const RECENT: &str = "recent";
    pub const CALENDAR: &str = "calendar";
    pub const ALL: &str = "all";
}

impl Config {
    /// Use query by the ID, or fallback to one of the built-ins.
    pub fn query(&self, query_id: &str) -> Result<QueryData<'_>> {
        unwrap_none_or!(self.queries.get(query_id), q, {
            return Ok(QueryData {
                sorting: &q.sorting,
                filter: &q.filter,
                group_by: &q.group_by,
                index: q.index,
            });
        });

        Ok(match query_id {
            bq::BACKLOG => self.query_backlog(),
            bq::UPCOMING => self.query_upcoming(),
            bq::CURRENT => self.query_current(),
            bq::DUE_TODAY => self.query_due_today(),
            bq::STARTED => self.query_started(),
            bq::DONE_TODAY => self.query_done_today(),
            bq::RECENT => self.query_recent(),
            bq::CALENDAR => self.query_calendar(),
            bq::ALL => self.query_all(),

            _ => bail!("Query '{query_id}' not defined"),
        })
    }
}

impl Config {
    fn query_all(&self) -> QueryData<'_> {
        QueryData {
            sorting: "end+ created+",
            filter: "",
            group_by: "end at 0:00",
            index: IndexType::All,
        }
    }

    fn query_recent(&self) -> QueryData<'_> {
        QueryData {
            sorting: "modified+",
            filter: "modified > -14d",
            group_by: "modified at 0:00",
            index: IndexType::All,
        }
    }

    fn query_backlog(&self) -> QueryData<'_> {
        QueryData {
            sorting: "urgency+",
            filter: "(when or someday) >= 365d and (due or someday) >= 365d and status != 'started'",
            group_by: "",
            index: IndexType::Active,
        }
    }

    fn query_upcoming(&self) -> QueryData<'_> {
        QueryData {
            sorting: "urgency+",
            filter: "((when >= 3d and when < 365d and not due) or (due >= 3d and due < 365d)) and status != 'started'",
            group_by: "",
            index: IndexType::Active,
        }
    }

    fn query_current(&self) -> QueryData<'_> {
        QueryData {
            sorting: "urgency+",
            filter: "((when < 3d and not due) or (due >= tomorrow and due < 3d)) and status != 'started'",
            group_by: "",
            index: IndexType::Active,
        }
    }

    fn query_due_today(&self) -> QueryData<'_> {
        QueryData {
            sorting: "urgency+",
            filter: "due < tomorrow and status != 'started'",
            group_by: "",
            index: IndexType::Active,
        }
    }

    fn query_started(&self) -> QueryData<'_> {
        QueryData {
            sorting: "urgency+",
            filter: "status == 'started'",
            group_by: "",
            index: IndexType::Active,
        }
    }

    fn query_done_today(&self) -> QueryData<'_> {
        QueryData {
            sorting: "end+",
            filter: "end >= today and status == 'completed'",
            group_by: "",
            index: IndexType::All,
        }
    }

    fn query_calendar(&self) -> QueryData<'_> {
        QueryData {
            sorting: "due-",
            filter: "due",
            group_by: "due at 0:00",
            index: IndexType::Active,
        }
    }
}
