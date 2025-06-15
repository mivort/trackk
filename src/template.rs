use minijinja::ErrorKind;
use std::cell::RefCell;

use crate::prelude::*;

/// Rendering template lazy loader.
#[derive(Default)]
pub struct Templates<'env> {
    pub j2: RefCell<minijinja::Environment<'env>>,
}

impl<'env> Templates<'env> {
    /// Check template ID existence, if template doesn't exist yet - load and parse it.
    pub fn load_template(&self, template: &'env str) -> Result<()> {
        let mut j2 = self.j2.borrow_mut();
        let err = unwrap_err_or!(j2.get_template(template), _, { return Ok(()) });

        if !matches!(err.kind(), ErrorKind::TemplateNotFound) {
            return Err(anyhow!(err));
        }

        match template {
            "next" => j2.add_template(template, "{{ title }}\n\n")?,
            "all" => j2.add_template(template, "{{ title }}\n\n")?,

            // TODO: P3: resolve external templates
            _ => panic!(),
        }

        Ok(())
    }
}
