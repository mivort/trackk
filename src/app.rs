use std::cell::{Ref, RefCell, RefMut};
use std::collections::HashMap;
use std::rc::Rc;

use once_cell::unsync::OnceCell;

use crate::args::FilterArgs;
use crate::dateexp::parse_local_exp;
use crate::entry::Entry;
use crate::prelude::*;
use crate::{bucket, config, filter, index, sort, templating, token};

/// App context which provides on-demand loading of data.
#[derive(Default)]
pub struct App<'env> {
    /// Application config.
    pub config: config::Config,

    /// Current global entry filter.
    pub filter: filter::QueryFilter,

    /// Sorting override.
    pub sort: Vec<sort::SortingRule>,

    /// Global entry count limit.
    pub limit: usize,

    /// Skip provided number of topmost filtered results.
    pub skip: usize,

    /// Tera templates reference.
    pub templates: RefCell<templating::Templates<'env>>,

    /// Parsed entries cache.
    pub cache: RefCell<HashMap<String, Rc<bucket::Bucket>>>,

    /// UTC timestamp during the init.
    pub ts: i64,

    /// Active entry index.
    index: RefCell<index::Index>,

    /// Parsed urgency expression.
    urgency: OnceCell<Vec<token::Token>>,
}

impl<'env> App<'env> {
    pub fn new(config: config::Config) -> Self {
        Self {
            config,
            ts: time::UtcDateTime::now().unix_timestamp(),
            limit: usize::MAX,
            ..Default::default()
        }
    }

    /// Set app-level options taken from arguments.
    pub fn merge_filter_args(&mut self, args: &FilterArgs) -> Result<()> {
        let mut filter = std::mem::take(&mut self.filter);
        filter::merge_filter_args(&mut filter, args, self)?;

        self.filter = filter;

        if let Some(query) = &args.query {
            let sort = self.config.query(query)?.sorting;
            self.sort = sort::parse_rules(sort)?;
        }

        if let Some(sort) = &args.sort {
            self.sort = sort::parse_rules(sort)?;
        }

        self.limit = args.limit.min(self.limit);
        self.skip = args.skip.max(self.skip);

        Ok(())
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
    pub fn urgency(&self) -> Result<&Vec<token::Token>> {
        self.urgency.get_or_try_init(|| {
            let mut urgency = Vec::new();
            let formula = self.config.values.urgency_formula();
            parse_local_exp(formula, self, &mut urgency)
                .with_context(|| format!("Unable to parse urgency formula: '{}'", formula))?;
            Ok(urgency)
        })
    }

    /// Check if filter output was defined.
    pub fn has_range(&self) -> bool {
        self.skip > 0 || self.limit < usize::MAX
    }

    /// Apply app-level sorting and range trim.
    pub fn apply_range(&self, entries: &mut Vec<(Entry, Rc<str>)>) {
        entries.truncate(entries.len().saturating_sub(self.skip));
        entries.drain(..(entries.len().saturating_sub(self.limit)));
    }
}
