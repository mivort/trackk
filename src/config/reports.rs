use serde_derive::Deserialize;
use std::borrow::Cow;

use super::Config;
use crate::config::query::builtin_queries as bq;
use crate::prelude::*;
use crate::templates::builtin_templates as bt;

/// Report configuration which contains array of report sections.
#[derive(Deserialize, Default, Clone)]
#[cfg_attr(test, derive(Debug, PartialEq, Eq))]
pub struct ReportConfig {
    /// Template sections, each with its own query and template.
    pub sections: Vec<SectionConfig>,

    /// List of templates to pre-load (so those can be used for inheritance).
    pub preload: Vec<Box<str>>, // TODO: P2: support list of base templates
}

/// Report section defined by filter and template.
#[derive(Deserialize, Default, Clone)]
#[cfg_attr(test, derive(Debug, PartialEq, Eq))]
pub struct SectionConfig {
    /// Section header template.
    #[serde(default)]
    pub header: Box<str>,

    /// Group header template.
    #[serde(default)]
    pub group: Box<str>,

    /// Name of tera template file used for section output.
    #[serde(default)]
    pub template: Box<str>,

    /// Query to use for report section.
    #[serde(default)]
    pub query: Box<str>,

    /// Section title.
    #[serde(default)]
    pub title: Box<str>,
}

impl Config {
    /// Fetch report instance from provided ID.
    pub fn report(&self, report: &str) -> Result<Cow<'_, ReportConfig>> {
        if let Some(report) = self.reports.get(report) {
            return Ok(Cow::Borrowed(report));
        }

        match report {
            "all" => Ok(Cow::Owned(self.report_all())),
            "next" => Ok(Cow::Owned(self.report_next())),
            "calendar" => Ok(Cow::Owned(self.report_calendar())),
            "recent" => Ok(Cow::Owned(self.report_recent())),
            "info" => Ok(Cow::Owned(self.report_info())),
            _ => bail!("Report '{report}' not found"),
        }
    }

    /// Default report format.
    pub fn report_next(&self) -> ReportConfig {
        ReportConfig {
            sections: vec![
                SectionConfig {
                    title: "Backlog".into(),
                    query: bq::BACKLOG.into(),
                    header: bt::HEADER.into(),
                    group: "".into(),
                    template: bt::NEXT.into(),
                },
                SectionConfig {
                    title: "Upcoming".into(),
                    query: bq::UPCOMING.into(),
                    header: bt::HEADER.into(),
                    group: "".into(),
                    template: bt::NEXT.into(),
                },
                SectionConfig {
                    title: "Current".into(),
                    query: bq::CURRENT.into(),
                    header: bt::HEADER.into(),
                    group: "".into(),
                    template: bt::NEXT.into(),
                },
                SectionConfig {
                    title: "Due today".into(),
                    query: bq::DUE_TODAY.into(),
                    header: bt::HEADER.into(),
                    group: "".into(),
                    template: bt::NEXT.into(),
                },
                SectionConfig {
                    title: "Started".into(),
                    query: bq::STARTED.into(),
                    header: bt::HEADER.into(),
                    group: "".into(),
                    template: bt::NEXT.into(),
                },
                SectionConfig {
                    title: "Done today".into(),
                    query: bq::DONE_TODAY.into(),
                    header: bt::HEADER.into(),
                    group: "".into(),
                    template: bt::NEXT.into(),
                },
            ],
            preload: vec![bt::UTILS.into()],
        }
    }
}

impl Config {
    /// Report which displays all entries.
    fn report_all(&self) -> ReportConfig {
        ReportConfig {
            sections: vec![SectionConfig {
                title: "All entries".into(),
                query: bq::ALL.into(),
                header: bt::HEADER.into(),
                group: bt::HEADER_DAY.into(),
                template: bt::ALL.into(),
            }],
            preload: vec![bt::UTILS.into()],
        }
    }

    /// Report which displays all entries.
    fn report_recent(&self) -> ReportConfig {
        ReportConfig {
            sections: vec![SectionConfig {
                title: "Recent entries".into(),
                query: bq::RECENT.into(),
                header: bt::HEADER.into(),
                group: bt::HEADER_DAY.into(),
                template: bt::ALL.into(),
            }],
            preload: vec![bt::UTILS.into()],
        }
    }

    /// Report which only shows 'due' entries grouped by day.
    fn report_calendar(&self) -> ReportConfig {
        ReportConfig {
            sections: vec![SectionConfig {
                title: "Calendar".into(),
                query: bq::CALENDAR.into(),
                header: bt::HEADER.into(),
                group: bt::HEADER_DAY.into(),
                template: bt::CALENDAR.into(),
            }],
            preload: vec![bt::UTILS.into()],
        }
    }

    /// Report which displays all entries.
    fn report_info(&self) -> ReportConfig {
        ReportConfig {
            sections: vec![SectionConfig {
                query: bt::ALL.into(),
                template: bt::ENTRY.into(),

                title: "".into(),
                header: "".into(),
                group: "".into(),
            }],
            preload: vec![bt::UTILS.into()],
        }
    }
}
