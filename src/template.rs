use std::cell::RefCell;

use crate::prelude::*;

/// Rendering template lazy loader.
#[derive(Default)]
pub struct Templates {
    _tera: RefCell<tera::Tera>,
}

impl Templates {
    pub fn template(&self) -> Result<()> {
        Ok(())
    }
}
