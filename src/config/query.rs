use serde_derive::Deserialize;

use super::Config;
use crate::prelude::*;

/// Query config which defines filter expression and sorting columns.
#[derive(Deserialize, Default, Clone)]
#[cfg_attr(test, derive(Debug, PartialEq, Eq))]
pub struct QueryConfig {
    /// Sorting direction.
    #[serde(default)]
    pub sorting: Box<str>,

    /// Section filter parameters.
    #[serde(default)]
    pub filter: Box<str>,

    /// Index to use when query is called.
    pub index: IndexType,
}

/// Reference to query data.
pub struct QueryData<'a> {
    pub sorting: &'a str,
    pub filter: &'a str,
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
    pub fn query(&self, query_id: &str) -> Option<QueryData> {
        unwrap_none_or!(self.queries.get(query_id), q, {
            return Some(QueryData {
                sorting: &q.sorting,
                filter: &q.filter,
                index: q.index,
            });
        });

        Some(match query_id {
            "backlog" => self.query_backlog(),
            "upcoming" => self.query_upcoming(),
            "current" => self.query_current(),
            "overdue" => self.query_overdue(),
            "started" => self.query_started(),
            "done_today" => self.query_done_today(),
            "recent" => self.query_recent(),
            "all" => self.query_all(),
            _ => return None,
        })
    }
}

impl Config {
    fn query_all(&self) -> QueryData {
        QueryData {
            sorting: "end+ created+",
            filter: "",
            index: IndexType::All,
        }
    }

    fn query_recent(&self) -> QueryData {
        QueryData {
            sorting: "modified+",
            filter: "modified > -14d",
            index: IndexType::All,
        }
    }

    fn query_backlog(&self) -> QueryData {
        QueryData {
            sorting: "urgency+",
            filter: "(when or someday) >= 365d and (due or someday) >= 365d and status != started",
            index: IndexType::Active,
        }
    }

    fn query_upcoming(&self) -> QueryData {
        QueryData {
            sorting: "urgency+",
            filter: "((when >= 3d and when < 365d and due:false) or (due >= 3d and due < 365d)) and status != started",
            index: IndexType::Active,
        }
    }

    fn query_current(&self) -> QueryData {
        QueryData {
            sorting: "urgency+",
            filter: "((when < 3d and due:false) or (due >= now and due < 3d)) and status != started",
            index: IndexType::Active,
        }
    }

    fn query_overdue(&self) -> QueryData {
        QueryData {
            sorting: "urgency+",
            filter: "due < now and status != started",
            index: IndexType::Active,
        }
    }

    fn query_started(&self) -> QueryData {
        QueryData {
            sorting: "urgency+",
            filter: "status == started",
            index: IndexType::Active,
        }
    }

    fn query_done_today(&self) -> QueryData {
        QueryData {
            sorting: "end+",
            filter: "end >= today and status == completed",
            index: IndexType::All,
        }
    }
}
