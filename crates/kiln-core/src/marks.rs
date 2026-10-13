//! Named marks mapped across edits.

use std::collections::BTreeMap;

use crate::transaction::{Assoc, Transaction};

/// A collection of named character positions mapped through transactions.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Marks {
    marks: BTreeMap<char, usize>,
}

impl Marks {
    /// Creates an empty set of marks.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            marks: BTreeMap::new(),
        }
    }

    /// Sets or updates a named mark position.
    pub fn set(&mut self, name: char, pos: usize) {
        self.marks.insert(name, pos);
    }

    /// Retrieves the position of a named mark.
    #[must_use]
    pub fn get(&self, name: char) -> Option<usize> {
        self.marks.get(&name).copied()
    }

    /// Removes a named mark.
    pub fn remove(&mut self, name: char) -> Option<usize> {
        self.marks.remove(&name)
    }

    /// Returns an iterator over all marks.
    pub fn iter(&self) -> impl Iterator<Item = (&char, &usize)> {
        self.marks.iter()
    }

    /// Returns the number of active marks.
    #[must_use]
    pub fn len(&self) -> usize {
        self.marks.len()
    }

    /// Returns `true` if there are no marks.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.marks.is_empty()
    }

    /// Maps all mark positions through a transaction with `Assoc::After`.
    pub fn map(&mut self, tx: &Transaction) {
        for pos in self.marks.values_mut() {
            *pos = tx.map_pos(*pos, Assoc::After);
        }
    }
}
