//! Terminal-independent key event representations.

use std::fmt;

/// Modifiers applied to a key press.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct KeyModifiers {
    /// Control key is held.
    pub ctrl: bool,
    /// Alt / Option key is held.
    pub alt: bool,
    /// Shift key is held.
    pub shift: bool,
}

impl KeyModifiers {
    /// No modifiers held.
    pub const NONE: Self = Self {
        ctrl: false,
        alt: false,
        shift: false,
    };

    /// Only Control is held.
    pub const CTRL: Self = Self {
        ctrl: true,
        alt: false,
        shift: false,
    };

    /// Only Alt is held.
    pub const ALT: Self = Self {
        ctrl: false,
        alt: true,
        shift: false,
    };

    /// Only Shift is held.
    pub const SHIFT: Self = Self {
        ctrl: false,
        alt: false,
        shift: true,
    };

    /// Returns `true` if no modifiers are active.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        !self.ctrl && !self.alt && !self.shift
    }
}

/// Identifies the key pressed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum KeyCode {
    /// Regular unicode character.
    Char(char),
    /// Enter / Return key.
    Enter,
    /// Tab key.
    Tab,
    /// Backspace key.
    Backspace,
    /// Escape key.
    Esc,
    /// Left arrow.
    Left,
    /// Right arrow.
    Right,
    /// Up arrow.
    Up,
    /// Down arrow.
    Down,
    /// Home key.
    Home,
    /// End key.
    End,
    /// Page Up key.
    PageUp,
    /// Page Down key.
    PageDown,
    /// Delete key.
    Delete,
    /// Function key F1–F12.
    F(u8),
    /// Null key.
    Null,
}

/// A terminal-independent key press event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct KeyEvent {
    /// The key code.
    pub code: KeyCode,
    /// Active modifiers.
    pub modifiers: KeyModifiers,
}

impl KeyEvent {
    /// Creates a key event with no modifiers.
    #[must_use]
    pub const fn new(code: KeyCode) -> Self {
        Self {
            code,
            modifiers: KeyModifiers::NONE,
        }
    }

    /// Creates a key event from a single character with no modifiers.
    #[must_use]
    pub const fn char(c: char) -> Self {
        Self {
            code: KeyCode::Char(c),
            modifiers: KeyModifiers::NONE,
        }
    }

    /// Creates a key event with Control modifier.
    #[must_use]
    pub const fn ctrl(code: KeyCode) -> Self {
        Self {
            code,
            modifiers: KeyModifiers::CTRL,
        }
    }

    /// Creates a key event with Alt modifier.
    #[must_use]
    pub const fn alt(code: KeyCode) -> Self {
        Self {
            code,
            modifiers: KeyModifiers::ALT,
        }
    }

    /// Parses a standard Vim-style key notation string (e.g. `<C-w>`, `<Esc>`, `j`, `<A-x>`, `Enter`).
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        if s.is_empty() {
            return None;
        }

        // Bracketed special keys e.g. <C-a>, <Esc>, <CR>, <Tab>, <BS>
        if s.starts_with('<') && s.ends_with('>') && s.len() > 2 {
            let inner = &s[1..s.len() - 1];
            let mut parts = inner.split('-').collect::<Vec<_>>();

            let mut modifiers = KeyModifiers::NONE;
            while parts.len() > 1 {
                let m = parts.remove(0);
                match m.to_uppercase().as_str() {
                    "C" | "CTRL" => modifiers.ctrl = true,
                    "A" | "ALT" | "M" => modifiers.alt = true,
                    "S" | "SHIFT" => modifiers.shift = true,
                    _ => return None,
                }
            }

            let key_str = parts.first()?;
            let code = match key_str.to_lowercase().as_str() {
                "esc" | "escape" => KeyCode::Esc,
                "enter" | "cr" | "return" => KeyCode::Enter,
                "tab" => KeyCode::Tab,
                "bs" | "backspace" => KeyCode::Backspace,
                "del" | "delete" => KeyCode::Delete,
                "left" => KeyCode::Left,
                "right" => KeyCode::Right,
                "up" => KeyCode::Up,
                "down" => KeyCode::Down,
                "home" => KeyCode::Home,
                "end" => KeyCode::End,
                "pageup" | "pgup" => KeyCode::PageUp,
                "pagedown" | "pgdn" => KeyCode::PageDown,
                "space" => KeyCode::Char(' '),
                other => {
                    if let Some(f_num) = other.strip_prefix('f') {
                        if let Ok(num) = f_num.parse::<u8>() {
                            KeyCode::F(num)
                        } else {
                            return None;
                        }
                    } else if other.chars().count() == 1 {
                        KeyCode::Char(other.chars().next()?)
                    } else {
                        return None;
                    }
                }
            };

            Some(Self { code, modifiers })
        } else {
            // Unbracketed key
            match s {
                "Enter" | "Return" => Some(Self::new(KeyCode::Enter)),
                "Tab" => Some(Self::new(KeyCode::Tab)),
                "Backspace" => Some(Self::new(KeyCode::Backspace)),
                "Esc" => Some(Self::new(KeyCode::Esc)),
                "Delete" => Some(Self::new(KeyCode::Delete)),
                "Left" => Some(Self::new(KeyCode::Left)),
                "Right" => Some(Self::new(KeyCode::Right)),
                "Up" => Some(Self::new(KeyCode::Up)),
                "Down" => Some(Self::new(KeyCode::Down)),
                _ => {
                    if s.chars().count() == 1 {
                        Some(Self::char(s.chars().next()?))
                    } else {
                        None
                    }
                }
            }
        }
    }
}

impl fmt::Display for KeyEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut prefix = String::new();
        if self.modifiers.ctrl {
            prefix.push_str("C-");
        }
        if self.modifiers.alt {
            prefix.push_str("A-");
        }
        if self.modifiers.shift {
            prefix.push_str("S-");
        }

        match self.code {
            KeyCode::Char(c) => {
                if prefix.is_empty() {
                    write!(f, "{c}")
                } else {
                    write!(f, "<{prefix}{c}>")
                }
            }
            KeyCode::Esc => write!(f, "<{prefix}Esc>"),
            KeyCode::Enter => write!(f, "<{prefix}Enter>"),
            KeyCode::Tab => write!(f, "<{prefix}Tab>"),
            KeyCode::Backspace => write!(f, "<{prefix}Backspace>"),
            KeyCode::Delete => write!(f, "<{prefix}Delete>"),
            KeyCode::Left => write!(f, "<{prefix}Left>"),
            KeyCode::Right => write!(f, "<{prefix}Right>"),
            KeyCode::Up => write!(f, "<{prefix}Up>"),
            KeyCode::Down => write!(f, "<{prefix}Down>"),
            KeyCode::Home => write!(f, "<{prefix}Home>"),
            KeyCode::End => write!(f, "<{prefix}End>"),
            KeyCode::PageUp => write!(f, "<{prefix}PageUp>"),
            KeyCode::PageDown => write!(f, "<{prefix}PageDown>"),
            KeyCode::F(n) => write!(f, "<{prefix}F{n}>"),
            KeyCode::Null => write!(f, "<Null>"),
        }
    }
}
