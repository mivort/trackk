use crate::config::Config;
use crate::prelude::*;

/// Call expansion rules on arguments.
pub fn pre_process_args(_config: &Config) -> Result<()> {
    let mut args = std::env::args().into_iter();
    while let Some(_arg) = args.next() {}

    Ok(())
}
