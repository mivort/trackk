use minijinja::ErrorKind;
use std::cell::{Cell, RefCell};

use crate::prelude::*;

/// Rendering template lazy loader.
pub struct Templates<'env> {
    pub j2: RefCell<minijinja::Environment<'env>>,

    /// Flag if initial lazy setup was done.
    init: Cell<bool>,
}

impl<'env> Default for Templates<'env> {
    fn default() -> Self {
        Self {
            j2: RefCell::new(minijinja::Environment::new()),
            init: Cell::new(false),
        }
    }
}

impl<'env> Templates<'env> {
    /// Initialize the templating environment.
    pub fn init(&self) {
        if self.init.get() {
            return
        }

        let mut j2 = self.j2.borrow_mut();
        j2.set_keep_trailing_newline(true);

        self.init.set(true);
    }

    /// Check template ID existence, if template doesn't exist yet - load and parse it.
    pub fn load_template(&self, template: &'env str) -> Result<()> {
        let mut j2 = self.j2.borrow_mut();
        let err = unwrap_err_or!(j2.get_template(template), _, { return Ok(()) });

        if !matches!(err.kind(), ErrorKind::TemplateNotFound) {
            return Err(anyhow!(err));
        }

        match template {
            "next" => j2.add_template(template, include_str!("../templates/row.jinja"))?,
            "all" => j2.add_template(template, include_str!("../templates/row.jinja"))?,

            // TODO: P3: resolve external templates
            _ => panic!(),
        }

        Ok(())
    }
}
