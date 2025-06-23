use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use serde_derive::Deserialize;
use time::PrimitiveDateTime;
use time::macros::format_description;

use crate::app::App;
use crate::issue::Issue;
use crate::prelude::*;

/// Taskwarrior export data format schema.
#[derive(Deserialize)]
#[allow(unused)]
struct TWData {
    uuid: Box<str>,
    description: Box<str>,

    #[serde(default)]
    entry: Box<str>,

    #[serde(default)]
    modified: Box<str>,

    #[serde(default)]
    due: Option<Box<str>>,

    #[serde(default)]
    end: Option<Box<str>>,

    #[serde(default)]
    status: Box<str>,

    #[serde(default)]
    tags: Vec<Box<str>>,

    #[serde(default)]
    annotations: Vec<TWAnnotation>,
    // TODO: P1: add 'depends' handling
    // TODO: P1: support uda import
}

#[derive(Deserialize)]
#[allow(unused)]
struct TWAnnotation {
    #[serde(default)]
    entry: Box<str>,

    #[serde(default)]
    description: Box<str>,
}

// Importer for Taskwarrior v2 JSON export format.
pub fn import_from_file(file: impl AsRef<Path>, app: &App) -> Result<()> {
    // TODO: P3: implement import from taskwarrior

    let file = File::open(file).context("Unable to open imported file")?;
    let buf = BufReader::new(file);

    let entries: Vec<TWData> = serde_json::from_reader(buf)?;
    import_entries(entries, app)
}

/// Iterate of array of TW entries and use bucket cache to avoid flushing on each change.
fn import_entries(entries: Vec<TWData>, app: &App) -> Result<()> {
    let _cache = &*app.cache.borrow_mut();

    let format = format_description!("[year][month][day]T[hour][minute][second]Z");
    let try_parse = |v: &Box<str>| {
        PrimitiveDateTime::parse(&v, format).map(|t| t.assume_utc().unix_timestamp())
    };

    for e in entries {
        let _imported = Issue {
            id: e.uuid,
            desc: e.description.into_string(),
            status: e.status.into_string(),
            tags: e.tags.into_iter().map(|e| e.into_string()).collect(),
            created: try_parse(&e.entry)?,
            modified: try_parse(&e.modified)?,
            due: e.due.map_or(Ok(None), |v| try_parse(&v).map(Some))?,
            end: e.end.map_or(Ok(None), |v| try_parse(&v).map(Some))?,
            ..Default::default()
        };
    }

    Ok(())
}

#[cfg(test)]
const TW_SAMPLE: &str = r#"
[ {"id":0,"description":"test task 1","due":"20250529T183137Z","end":"20250529T183137Z","entry":"20250529T183137Z","modified":"20250529T183137Z","status":"completed","uuid":"ad2b15f0-af23-4fdc-9bf4-3be8a43a529f","tags":["test1"],"urgency":15.0932}
, {"id":0,"description":"test task 2","due":"20250529T230726Z","end":"20250529T230726Z","entry":"20250529T230726Z","modified":"20250529T230726Z","status":"completed","uuid":"291a768d-d395-4d7b-bec9-a69724f84dca","tags":["test2"],"urgency":15.0926}
, {"id":0,"description":"test task 3","due":"20250530T105355Z","end":"20250530T105355Z","entry":"20250530T105355Z","modified":"20250530T105355Z","status":"completed","uuid":"21b810a5-1b52-4361-95aa-89d7330a0138","tags":["test3"],"urgency":15.0926}
]
"#;

#[test]
fn parse_piece() {
    let _entries: Vec<TWData> = serde_json::from_str(TW_SAMPLE).unwrap();
}
