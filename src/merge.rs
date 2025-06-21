use crate::args::MergeArgs;

use crate::{bucket::Bucket, prelude::*};

/// Implement 3-way merge driver for once ancestor and two JSON buckets.
pub fn merge_driver(args: &MergeArgs) -> Result<()> {
    // TODO: P3: implement merge driver

    let ancestor = Bucket::from_full_path(&args.ancestor)?;
    let mut ours = Bucket::from_full_path(&args.ours)?;
    let theirs = Bucket::from_full_path(&args.theirs)?;

    merge_buckets(&ancestor, &theirs, &mut ours);

    // TODO: write the result back

    Ok(())
}

/// Perform bucket merge on the current version.
fn merge_buckets(_ancestor: &Bucket, _theirs: &Bucket, _ours: &mut Bucket) {}
