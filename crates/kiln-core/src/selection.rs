//! Selection and ranges.
//!
//! Character-offset based ranges and normalized selection lists.

use std::cmp::{max, min};

use crate::error::CoreError;
use crate::transaction::{Assoc, ChangeSet};

/// A single selection range defined by an anchor and head offset in characters.
///
/// The anchor is where selection started; the head is where the cursor currently is.
/// `anchor` may be greater than `head` (backwards selection).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Range {
    /// Starting anchor position (in characters).
    pub anchor: usize,
    /// Cursor head position (in characters).
    pub head: usize,
}

impl Range {
    /// Creates a new `Range` with the given anchor and head.
    #[must_use]
    pub const fn new(anchor: usize, head: usize) -> Self {
        Self { anchor, head }
    }

    /// Creates an empty (cursor point) `Range` at `pos`.
    #[must_use]
    pub const fn point(pos: usize) -> Self {
        Self {
            anchor: pos,
            head: pos,
        }
    }

    /// Returns the minimum character offset in this range.
    #[must_use]
    pub fn from(&self) -> usize {
        min(self.anchor, self.head)
    }

    /// Returns the maximum character offset in this range.
    #[must_use]
    pub fn to(&self) -> usize {
        max(self.anchor, self.head)
    }

    /// Returns `true` if this range represents a zero-width cursor point.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.anchor == self.head
    }

    /// Returns the length of this range in characters.
    #[must_use]
    pub fn len(&self) -> usize {
        self.to() - self.from()
    }

    /// Returns `true` if this range contains `pos` (inclusive of `from` and `to`).
    #[must_use]
    pub fn contains(&self, pos: usize) -> bool {
        pos >= self.from() && pos <= self.to()
    }

    /// Maps this range through a `ChangeSet`.
    ///
    /// Mapping rules:
    /// - An empty range (cursor point) maps with `Assoc::After`.
    /// - For a non-empty range, the start boundary (`from()`) maps with `Assoc::After` and
    ///   the end boundary (`to()`) maps with `Assoc::Before` so that text inserted exactly
    ///   at the boundaries stays outside the selection range.
    /// - Deletions covering boundaries collapse to the deletion boundary.
    /// - The original direction (`anchor <= head` or `head < anchor`) is preserved.
    #[must_use]
    pub fn map(&self, changes: &ChangeSet) -> Self {
        if self.is_empty() {
            let mapped = changes.map_pos(self.anchor, Assoc::After);
            return Self::point(mapped);
        }

        let is_forward = self.anchor <= self.head;
        let start = self.from();
        let end = self.to();

        let new_start = changes.map_pos(start, Assoc::After);
        let new_end = changes.map_pos(end, Assoc::Before);

        // Ensure new_end >= new_start
        let (final_start, final_end) = if new_start <= new_end {
            (new_start, new_end)
        } else {
            (new_start, new_start)
        };

        if is_forward {
            Self::new(final_start, final_end)
        } else {
            Self::new(final_end, final_start)
        }
    }
}

/// A non-empty list of ranges representing the user's active selections.
///
/// Invariants:
/// - Contains at least one range.
/// - Ranges are always sorted in ascending order of `range.from()`.
/// - Overlapping or adjacent ranges are merged.
/// - `primary` index points to a valid range in the list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Selections {
    ranges: Vec<Range>,
    primary: usize,
}

impl Selections {
    /// Creates a new `Selections` list and normalizes it.
    ///
    /// # Errors
    /// Returns `CoreError::EmptySelections` if `ranges` is empty, or
    /// `CoreError::PrimarySelectionOutOfBounds` if `primary >= ranges.len()`.
    pub fn new(ranges: Vec<Range>, primary: usize) -> Result<Self, CoreError> {
        if ranges.is_empty() {
            return Err(CoreError::EmptySelections);
        }
        if primary >= ranges.len() {
            return Err(CoreError::PrimarySelectionOutOfBounds {
                primary,
                len: ranges.len(),
            });
        }
        let (normalized, new_primary) = normalize_ranges(ranges, primary);
        Ok(Self {
            ranges: normalized,
            primary: new_primary,
        })
    }

    /// Creates a single point selection at `pos`.
    #[must_use]
    pub fn point(pos: usize) -> Self {
        Self {
            ranges: vec![Range::point(pos)],
            primary: 0,
        }
    }

    /// Returns a slice of all ranges in this selection list.
    #[must_use]
    pub fn ranges(&self) -> &[Range] {
        &self.ranges
    }

    /// Returns a reference to the primary selection range.
    #[must_use]
    pub fn primary(&self) -> &Range {
        &self.ranges[self.primary]
    }

    /// Returns the index of the primary selection range.
    #[must_use]
    pub fn primary_index(&self) -> usize {
        self.primary
    }

    /// Maps all ranges through a `ChangeSet` and normalizes the resulting list.
    #[must_use]
    pub fn map(&self, changes: &ChangeSet) -> Self {
        let mapped_ranges: Vec<Range> = self.ranges.iter().map(|r| r.map(changes)).collect();
        let (normalized, new_primary) = normalize_ranges(mapped_ranges, self.primary);
        Self {
            ranges: normalized,
            primary: new_primary,
        }
    }
}

/// Normalizes a list of ranges:
/// - Sorts by `range.from()`
/// - Merges overlapping and touching ranges
/// - Preserves the primary selection index
fn normalize_ranges(ranges: Vec<Range>, primary: usize) -> (Vec<Range>, usize) {
    if ranges.len() <= 1 {
        return (ranges, 0);
    }

    // Attach original index to track primary across sorting and merging
    let mut indexed: Vec<(usize, Range)> = ranges.into_iter().enumerate().collect();
    indexed.sort_by_key(|(_, r)| r.from());

    let mut merged: Vec<(bool, Range)> = Vec::with_capacity(indexed.len());
    let (first_idx, first_range) = indexed[0];
    merged.push((first_idx == primary, first_range));

    for (orig_idx, range) in indexed.into_iter().skip(1) {
        let is_prim = orig_idx == primary;
        let last_idx = merged.len() - 1;
        let (last_is_prim, last_range) = &mut merged[last_idx];

        // Overlapping or touching ranges:
        // touching: last_range.to() >= range.from()
        if last_range.to() >= range.from() {
            let combined_from = min(last_range.from(), range.from());
            let combined_to = max(last_range.to(), range.to());

            // If one was primary, merged is primary
            *last_is_prim = *last_is_prim || is_prim;

            // Preserve direction of primary, or fallback to forward
            let forward = if is_prim {
                range.anchor <= range.head
            } else {
                last_range.anchor <= last_range.head
            };

            *last_range = if forward {
                Range::new(combined_from, combined_to)
            } else {
                Range::new(combined_to, combined_from)
            };
        } else {
            merged.push((is_prim, range));
        }
    }

    let mut final_ranges = Vec::with_capacity(merged.len());
    let mut final_primary = 0;

    for (i, (is_prim, r)) in merged.into_iter().enumerate() {
        if is_prim {
            final_primary = i;
        }
        final_ranges.push(r);
    }

    (final_ranges, final_primary)
}
