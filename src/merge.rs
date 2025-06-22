use std::collections::BTreeMap;

use crate::args::MergeArgs;

use crate::issue::Issue;
use crate::{bucket::Bucket, prelude::*};

/// Implement 3-way merge driver for once ancestor and two JSON buckets.
pub fn merge_driver(args: &MergeArgs) -> Result<()> {
    // TODO: P3: implement merge driver

    let ancestor = Bucket::from_full_path(&args.ancestor)?;
    let ours = Bucket::from_full_path(&args.ours)?;
    let theirs = Bucket::from_full_path(&args.theirs)?;

    merge_buckets(ancestor, theirs, ours);

    // TODO: write the result back

    Ok(())
}

/// Perform bucket merge on the current version.
fn merge_buckets(mut ancestor: Bucket, theirs: Bucket, ours: Bucket) {
    let mut index = BTreeMap::<Box<str>, Issue>::new();

    for entry in ours.entries.into_iter() {
        index.insert(entry.id.clone(), entry);
    }

    for incoming in theirs.entries.into_iter() {
        let entry = index.get_mut(&incoming.id);
        if let Some(entry) = entry {
            let ancestor = ancestor.entries.iter_mut().find(|a| a.id == entry.id);
            if let Some(ancestor) = ancestor {
                merge_3way(entry, std::mem::take(ancestor), incoming);
            } else {
                merge_2way(entry, incoming);
            }
        } else {
            index.insert(incoming.id.clone(), incoming);
        }
    }
}

/// Take ancestor, incoming change and write result in the output.
fn merge_3way(ours: &mut Issue, parent: Issue, theirs: Issue) {
    let their_newer = ours.modified < theirs.modified;

    merge_field(&mut ours.title, parent.title, theirs.title, their_newer);
    merge_field(&mut ours.tags, parent.tags, theirs.tags, their_newer);
    merge_field(&mut ours.parent, parent.parent, theirs.parent, their_newer);
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
