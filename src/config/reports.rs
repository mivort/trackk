use serde_derive::Deserialize;
use std::borrow::Cow;

use super::Config;
use crate::prelude::*;

/// Report configuration which contains array of report sections.
#[derive(Deserialize, Default, Clone)]
#[cfg_attr(test, derive(Debug, PartialEq, Eq))]
pub struct ReportConfig {
    /// Template sections, each with its own query and template.
    pub sections: Vec<SectionConfig>,

    /// List of templates to pre-load (so those can be used for inheritance).
    #[allow(unused)]
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
                    query: "backlog".into(),
                    title: "Backlog".into(),
                    header: "header".into(),
                    group: "".into(),
                    template: "next".into(),
                },
                SectionConfig {
                    query: "upcoming".into(),
                    title: "Upcoming".into(),
                    header: "header".into(),
                    group: "".into(),
                    template: "next".into(),
                },
                SectionConfig {
                    query: "current".into(),
                    title: "Current".into(),
                    header: "header".into(),
                    group: "".into(),
                    template: "next".into(),
                },
                SectionConfig {
                    query: "due_today".into(),
                    title: "Due today".into(),
                    header: "header".into(),
                    group: "".into(),
                    template: "next".into(),
                },
                SectionConfig {
                    query: "started".into(),
                    title: "Started".into(),
                    header: "header".into(),
                    group: "".into(),
                    template: "next".into(),
                },
                SectionConfig {
                    query: "done_today".into(),
                    title: "Done today".into(),
                    header: "header".into(),
                    group: "".into(),
                    template: "next".into(),
                },
            ],
            preload: vec!["utils".into()],
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
                group: "header_day".into(),
                template: "all".into(),
            }],
            preload: vec!["utils".into()],
        }
    }

    /// Report which displays all entries.
    fn report_recent(&self) -> ReportConfig {
        ReportConfig {
            sections: vec![SectionConfig {
                query: "recent".into(),
                title: "Recent entries".into(),
                header: "header".into(),
                group: "header_day".into(),
                template: "all".into(),
            }],
            preload: vec!["utils".into()],
        }
    }

    /// Report which only shows 'due' entries grouped by day.
    fn report_calendar(&self) -> ReportConfig {
        ReportConfig {
            sections: vec![SectionConfig {
                query: "calendar".into(),
                title: "Calendar".into(),
                header: "header".into(),
                group: "header_day".into(),
                template: "calendar".into(),
            }],
            preload: vec!["utils".into()],
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
                group: "".into(),
            }],
            preload: vec!["utils".into()],
        }
    }
}
