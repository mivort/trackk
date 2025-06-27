use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use serde_derive::Deserialize;
use serde_json::Value;
use time::macros::format_description;
use time::{PrimitiveDateTime, UtcDateTime};

use crate::app::App;
use crate::bucket::Bucket;
use crate::entry::Entry;
use crate::{prelude::*, storage};

/// Taskwarrior export data format schema.
#[derive(Deserialize)]
#[allow(unused)]
struct TWData {
    #[serde(default)]
    id: Option<i64>,

    uuid: Box<str>,
    description: Box<str>,

    #[serde(default)]
    entry: Box<str>,

    #[serde(default)]
    modified: Box<str>,

    #[serde(default)]
    scheduled: Option<Box<str>>,

    #[serde(default)]
    due: Option<Box<str>>,

    #[serde(default)]
    end: Option<Box<str>>,

    #[serde(default)]
    start: Option<Box<str>>,

    #[serde(default)]
    wait: Option<Box<str>>,

    #[serde(default)]
    depends: Option<Vec<String>>,

    #[serde(default)]
    project: Option<Box<str>>,

    #[serde(default)]
    recur: Option<Box<str>>,

    #[serde(default)]
    parent: Option<Box<str>>,

    #[serde(default)]
    mask: Option<Box<str>>,

    #[serde(default)]
    imask: Option<f64>,

    #[serde(default)]
    status: Box<str>,

    #[serde(default)]
    tags: Vec<Box<str>>,

    #[serde(default)]
    annotations: Vec<TWAnnotation>,

    #[serde(flatten)]
    extra: HashMap<Box<str>, Value>,

    #[serde(default)]
    urgency: Option<f64>,
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
    // TODO: P2: implement import from stdin

    let file = File::open(file).context("Unable to open imported file")?;
    let buf = BufReader::new(file);

    let entries: Vec<TWData> = serde_json::from_reader(buf)?;
    import_entries(entries, app)
}

/// Iterate of array of TW entries and use bucket cache to avoid flushing on each change.
fn import_entries(entries: Vec<TWData>, app: &App) -> Result<()> {
    let format = format_description!("[year][month][day]T[hour][minute][second]Z");
    let try_parse =
        |v: &str| PrimitiveDateTime::parse(v, format).map(|t| t.assume_utc().unix_timestamp());

    let mut cache: HashMap<String, Bucket> = Default::default();

    let mut write_count = 0;
    let mut skip_count = 0;

    let mut project_count = 0;
    let mut annotations_count = 0;

    for e in entries {
        let mut imported = Entry {
            id: e.uuid,
            desc: e.description.into_string(),
            status: e.status.into_string(),
            tags: e.tags.into_iter().map(|e| e.into_string()).collect(),
            created: try_parse(&e.entry)?,
            modified: try_parse(&e.modified)?,
            when: e.scheduled.map_or(Ok(None), |v| try_parse(&v).map(Some))?,
            due: e.due.map_or(Ok(None), |v| try_parse(&v).map(Some))?,
            end: e.end.map_or(Ok(None), |v| try_parse(&v).map(Some))?,
            ..Default::default()
        };

        let meta = &mut imported.meta;

        if let Some(start) = e.start {
            meta.insert("start".into(), try_parse(&start)?.into());
        }
        if let Some(wait) = e.wait {
            meta.insert("wait".into(), try_parse(&wait)?.into());
        }
        if let Some(project) = e.project {
            meta.insert("project".into(), try_parse(&project)?.into());
            project_count += 1;
        }
        if let Some(depends) = e.depends {
            // TODO: convert into entry linking?
            meta.insert("depends".into(), depends.into());
        }
        if let Some(recur) = e.recur {
            meta.insert("recur".into(), recur.into_string().into());
        }
        if let Some(parent) = e.parent {
            meta.insert("parent".into(), parent.into_string().into());
        }
        if let Some(mask) = e.mask {
            meta.insert("mask".into(), mask.into_string().into());
        }
        if let Some(imask) = e.imask {
            meta.insert("imask".into(), imask.into());
        }

        for (k, v) in e.extra {
            imported.meta.insert(k.into_string(), v);
        }

        for ann in e.annotations {
            annotations_count += 1;

            imported.desc.push('\n');
            imported.desc.push_str(&ann.entry);
            imported.desc.push_str(" -- ");
            imported.desc.push_str(&ann.description);
        }

        let date = UtcDateTime::from_unix_timestamp(imported.created)?.date();
        let rel_path = storage::rel_path_by_date(&date);

        if let Some(bucket) = cache.get_mut(&rel_path) {
            if bucket.insert(imported).is_some() {
                skip_count += 1;
            } else {
                write_count += 1;
            }
        } else {
            let (mut bucket, _) = storage::fetch_new_bucket(&date, &app.config)?;
            if bucket.insert(imported).is_some() {
                skip_count += 1;
            } else {
                write_count += 1;
            }
            cache.insert(rel_path, bucket);
        }
    }

    for (rel_path, bucket) in cache {
        storage::write_bucket(&bucket, &rel_path, app)?;
    }

    if project_count > 0 {
        warn!("'Project' field is added as meta field (total: {project_count}).");
    }
    if annotations_count > 0 {
        warn!("{annotations_count} annotations are merged with descriptions.");
    }

    info!("Imported: {write_count}, skipped: {skip_count}");

    storage::refresh_index(app, false)
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
