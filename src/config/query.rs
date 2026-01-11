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
            "backlog" => self.query_backlog(),
            "upcoming" => self.query_upcoming(),
            "current" => self.query_current(),
            "due_today" => self.query_due_today(),
            "started" => self.query_started(),
            "done_today" => self.query_done_today(),
            "recent" => self.query_recent(),
            "calendar" => self.query_calendar(),
            "all" => self.query_all(),

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
            sorting: "due+",
            filter: "due >= -14d and due < 14d",
            group_by: "due at 0:00",
            index: IndexType::Active,
        }
    }
}
