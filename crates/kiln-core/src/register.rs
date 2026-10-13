//! Vim register storage and content types.

use std::collections::BTreeMap;

/// Value stored in a register.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegisterValue {
    /// Text content stored in the register.
    pub text: String,
    /// Whether the content was yanked/deleted linewise (e.g. `dd`, `yy`).
    pub is_linewise: bool,
}

impl RegisterValue {
    /// Creates a character-wise register value.
    #[must_use]
    pub fn character(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            is_linewise: false,
        }
    }

    /// Creates a line-wise register value.
    #[must_use]
    pub fn linewise(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            is_linewise: true,
        }
    }
}

/// Storage for Vim registers (unnamed `"`, black hole `_`, and named `a`..=`z`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Registers {
    unnamed: Option<RegisterValue>,
    named: BTreeMap<char, RegisterValue>,
}

impl Registers {
    /// Creates an empty register store.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Retrieves the content of the specified register.
    ///
    /// If `name` is `None` or `'"'`, returns the unnamed register.
    /// If `name` is `'_'` (black hole), returns `None`.
    #[must_use]
    pub fn get(&self, name: Option<char>) -> Option<&RegisterValue> {
        match name {
            None | Some('"') => self.unnamed.as_ref(),
            Some('_') => None, // Black hole register yields nothing
            Some(c) => {
                let lower = c.to_ascii_lowercase();
                self.named.get(&lower).or(self.unnamed.as_ref())
            }
        }
    }

    /// Sets the content of the specified register.
    ///
    /// If `name` is `None` or `'"'`, sets the unnamed register.
    /// If `name` is `'_'`, the value is discarded (black hole register).
    /// If `name` is uppercase (`'A'`..=`'Z'`), appends to the corresponding lowercase register.
    /// Setting any register also populates the unnamed register (unless black hole).
    pub fn set(&mut self, name: Option<char>, value: RegisterValue) {
        match name {
            Some('_') => {
                // Black hole register: discard
            }
            None | Some('"') => {
                self.unnamed = Some(value);
            }
            Some(c) if c.is_ascii_uppercase() => {
                let lower = c.to_ascii_lowercase();
                if let Some(existing) = self.named.get_mut(&lower) {
                    existing.text.push_str(&value.text);
                    existing.is_linewise = existing.is_linewise || value.is_linewise;
                    self.unnamed = Some(existing.clone());
                } else {
                    self.named.insert(lower, value.clone());
                    self.unnamed = Some(value);
                }
            }
            Some(c) => {
                let lower = c.to_ascii_lowercase();
                self.named.insert(lower, value.clone());
                self.unnamed = Some(value);
            }
        }
    }
}
