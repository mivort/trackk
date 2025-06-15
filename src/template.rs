use std::cell::RefCell;

use crate::prelude::*;

/// Rendering template lazy loader.
#[derive(Default)]
pub struct Templates<'a> {
    _jinja: RefCell<minijinja::Environment<'a>>,
}

impl<'a> Templates<'a> {
    pub fn template(&self, template: &str) -> Result<()> {
        match template {
            "next" => {}
            "all" => {}
            _ => todo!(), // TODO: P3: look for template in config and data directory
        }
        Ok(())
    }
}
