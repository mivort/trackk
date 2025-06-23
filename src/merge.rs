use std::collections::BTreeMap;
use std::fs;

use crate::args::MergeArgs;

use crate::issue::Issue;
use crate::{bucket::Bucket, prelude::*};

/// Implement 3-way merge driver for once ancestor and two JSON buckets.
pub fn merge_driver(args: &MergeArgs) -> Result<()> {
    info!("Merging conflict using 3-way strategy");

    let ancestor = Bucket::from_full_path(&args.ancestor)?;
    let ours = Bucket::from_full_path(&args.ours)?;
    let theirs = Bucket::from_full_path(&args.theirs)?;

    let merged = merge_buckets(ancestor, theirs, ours);

    let output = serde_json::to_string_pretty(&merged)?;
    fs::write(&args.ours, output).context("Unable to write merged bucket")
}

/// Perform bucket merge on the current version.
fn merge_buckets(mut ancestor: Bucket, theirs: Bucket, ours: Bucket) -> Bucket {
    let mut merged = BTreeMap::<Box<str>, Issue>::new();

    for entry in ours.entries.into_iter() {
        merged.insert(entry.id.clone(), entry);
    }

    for incoming in theirs.entries.into_iter() {
        let entry = merged.get_mut(&incoming.id);
        if let Some(entry) = entry {
            let ancestor = ancestor.entries.iter_mut().find(|a| a.id == entry.id);
            if let Some(ancestor) = ancestor {
                merge_3way(entry, std::mem::take(ancestor), incoming);
            } else {
                merge_2way(entry, incoming);
            }
        } else {
            merged.insert(incoming.id.clone(), incoming);
        }
    }

    Bucket {
        version: ours.version,
        entries: merged.into_values().collect(),
    }
}

/// Take ancestor, incoming change and write result in the output.
fn merge_3way(ours: &mut Issue, parent: Issue, theirs: Issue) {
    let their_newer = ours.modified < theirs.modified;

    merge_field(&mut ours.desc, parent.desc, theirs.desc, their_newer);
    merge_field(&mut ours.status, parent.status, theirs.status, their_newer);
    merge_field(&mut ours.tags, parent.tags, theirs.tags, their_newer);
    merge_field(&mut ours.linked, parent.linked, theirs.linked, their_newer);
    merge_field(&mut ours.repeat, parent.repeat, theirs.repeat, their_newer);
    merge_field(
        &mut ours.created,
        parent.created,
        theirs.created,
        their_newer,
    );
    merge_field(&mut ours.due, parent.due, theirs.due, their_newer);
    merge_field(&mut ours.end, parent.end, theirs.end, their_newer);

    // TODO: P2: merge meta fields

    if their_newer {
        ours.modified = theirs.modified
    }
}

/// Select entry with more recent modified timestamp.
fn merge_2way(ours: &mut Issue, incoming: Issue) {
    if ours.modified >= incoming.modified {
        return;
    }
    *ours = incoming; // TODO: P1: combine tags/meta?
}

/// Take incoming variant of field value in case if:
/// * Current variant didn't introduce the change to field, but incoming variant did.
/// * Both current and incoming variants introduced the change, but incoming did it later.
fn merge_field<T>(ours: &mut T, ancestor: T, incoming: T, theirs_newer: bool)
where
    T: Eq,
{
    if (ancestor == *ours && ancestor != incoming)
        || (ancestor != *ours && ancestor != incoming && theirs_newer)
    {
        *ours = incoming;
    }
}

#[test]
fn try_merge() {
    let parent = Bucket {
        version: 1,
        entries: vec![Issue {
            status: "pending".into(),
            desc: "old name".into(),
            modified: 5,
            ..Default::default()
        }],
    };

    let ours = Bucket {
        version: 1,
        entries: vec![Issue {
            status: "started".into(),
            desc: "new name".into(),
            modified: 10,
            ..Default::default()
        }],
    };

    let theirs = Bucket {
        version: 1,
        entries: vec![Issue {
            status: "complete".into(),
            desc: "old name".into(),
            modified: 15,
            ..Default::default()
        }],
    };

    let res = &merge_buckets(parent, theirs, ours).entries[0];
    assert_eq!(res.status, "complete");
    assert_eq!(res.desc, "new name");
}
