use serde_derive::Deserialize;
use std::borrow::Cow;

use super::Config;
use crate::prelude::*;

/// Report configuration which contains array of report sections.
#[derive(Deserialize, Default, Clone)]
#[cfg_attr(test, derive(Debug, PartialEq, Eq))]
pub struct ReportConfig {
    pub sections: Vec<SectionConfig>,
}

/// Report section defined by filter and template.
#[derive(Deserialize, Default, Clone)]
#[cfg_attr(test, derive(Debug, PartialEq, Eq))]
pub struct SectionConfig {
    /// Section header template.
    #[serde(default)]
    pub header: Box<str>,

    /// Name of tera template file used for section output.
    #[serde(default)]
    pub template: Box<str>,

    /// Query to use for report section.
    #[serde(default)]
    pub query: Box<str>,

    /// Section title.
    #[serde(default)]
    pub title: Box<str>,

    /// Grouping field.
    #[serde(default)]
    pub _grouping: Box<str>,
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
                    query: "backlog".into(),
                    title: "Backlog".into(),
                    header: "header".into(),
                    template: "next".into(),
                    _grouping: "".into(),
                },
                SectionConfig {
                    query: "upcoming".into(),
                    title: "Upcoming".into(),
                    header: "header".into(),
                    template: "next".into(),
                    _grouping: "".into(),
                },
                SectionConfig {
                    query: "current".into(),
                    title: "Current".into(),
                    header: "header".into(),
                    template: "next".into(),
                    _grouping: "".into(),
                },
                SectionConfig {
                    query: "overdue".into(),
                    title: "Overdue".into(),
                    header: "header".into(),
                    template: "next".into(),
                    _grouping: "".into(),
                },
                SectionConfig {
                    query: "started".into(),
                    title: "Started".into(),
                    header: "header".into(),
                    template: "next".into(),
                    _grouping: "".into(),
                },
                SectionConfig {
                    query: "done_today".into(),
                    title: "Done today".into(),
                    header: "header".into(),
                    template: "next".into(),
                    _grouping: "".into(),
                },
            ],
        }
    }
}

impl Config {
    /// Report which displays all entries.
    fn report_all(&self) -> ReportConfig {
        ReportConfig {
            sections: vec![SectionConfig {
                query: "all".into(),
                title: "All entries".into(),
                header: "header".into(),
                template: "all".into(),
                _grouping: "".into(),
            }],
        }
    }

    /// Report which displays all entries.
    fn report_recent(&self) -> ReportConfig {
        ReportConfig {
            sections: vec![SectionConfig {
                query: "recent".into(),
                title: "Recent entries".into(),
                header: "header".into(),
                template: "all".into(),
                _grouping: "".into(),
            }],
        }
    }

    /// Report which displays all entries.
    fn report_info(&self) -> ReportConfig {
        ReportConfig {
            sections: vec![SectionConfig {
                query: "all".into(),
                template: "entry".into(),

                title: "".into(),
                header: "".into(),
                _grouping: "".into(),
            }],
        }
    }
}
