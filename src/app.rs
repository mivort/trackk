use std::cell::{Ref, RefCell, RefMut};
use std::collections::HashMap;
use std::rc::Rc;

use crate::prelude::*;
use crate::{bucket, config, filter, index, sort, templating, token};

/// App context which provides on-demand loading of data.
#[derive(Default)]
pub struct App<'env> {
    /// Application config.
    pub config: config::Config,

    /// Current global entry filter.
    pub filter: filter::Filter,

    /// Sorting override.
    pub sort: Vec<sort::SortingRule>,

    /// Tera templates reference.
    pub templates: templating::Templates<'env>,

    /// Parsed entries cache.
    pub cache: RefCell<HashMap<String, Rc<bucket::Bucket>>>,

    /// UTC timestamp during the init.
    pub ts: i64,

    /// Active entry index.
    index: RefCell<index::Index>,

    /// Parsed urgency expression.
    _urgency: RefCell<Vec<token::Token>>,
}

impl<'env> App<'env> {
    pub fn new(config: config::Config) -> Self {
        Self {
            config,
            ts: time::UtcDateTime::now().unix_timestamp(),
            ..Default::default()
        }
    }

    /// Lazy load and access the active entry index.
    pub fn index(&self) -> Result<Ref<'_, index::Index>> {
        let index = self.index.borrow();
        if index.loaded() {
            return Ok(index);
        }
        drop(index);

        let mut index = self.index.borrow_mut();
        index.load(&self.config)?;
        drop(index);

        Ok(self.index.borrow())
    }

    /// Load load and get mutable reference to the index.
    pub fn index_mut(&self) -> Result<RefMut<'_, index::Index>> {
        let mut index = self.index.borrow_mut();
        if !index.loaded() {
            index.load(&self.config)?;
        }
        Ok(index)
    }

    /// Get reference to empty index.
    pub fn index_empty_mut(&self) -> Result<RefMut<'_, index::Index>> {
        let mut index = self.index.borrow_mut();
        index.load_path(&self.config)?;
        index.clear();

        Ok(index)
    }

    /// Convert start timestamp to time with offset.
    pub fn local_time(&self) -> Result<time::OffsetDateTime> {
        use time::*;

        let utc = UtcDateTime::from_unix_timestamp(self.ts)?;
        Ok(utc.to_offset(UtcOffset::current_local_offset()?))
    }

    /// Give reference to parsed urgency expression.
    pub fn _urgency(&self) -> Result<Ref<'_, Vec<token::Token>>> {
        let urgency = self._urgency.borrow();
        if !urgency.is_empty() {
            return Ok(urgency);
        }
        drop(urgency);

        let mut _urgency = self._urgency.borrow_mut();
        // TODO: P3: parse urgency
        drop(_urgency);

        Ok(self._urgency.borrow())
    }
}
