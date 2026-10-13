//! Terminal lifecycle, raw mode management, and panic hook installation.
//!
//! Guarantees that terminal state (raw mode, alternate screen, cursor, bracketed paste)
//! is cleanly restored upon normal program exit or unexpected panic.

use crate::error::UiError;
use crossterm::cursor::Show;
use crossterm::event::{
    DisableBracketedPaste, DisableFocusChange, EnableBracketedPaste, EnableFocusChange,
};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use std::io::{stdout, Write};
use std::sync::atomic::{AtomicBool, Ordering};

static TERMINAL_IN_RAW_MODE: AtomicBool = AtomicBool::new(false);

/// Restores the terminal to its standard state.
///
/// Safe to call multiple times; will not error or double-restore.
pub fn restore_terminal() {
    if TERMINAL_IN_RAW_MODE.swap(false, Ordering::SeqCst) {
        let _ = disable_raw_mode();
        let mut out = stdout();
        let _ = execute!(
            out,
            LeaveAlternateScreen,
            DisableBracketedPaste,
            DisableFocusChange,
            Show
        );
        let _ = out.flush();
    }
}

/// An RAII guard that configures the terminal upon creation and restores it when dropped.
#[derive(Debug)]
pub struct TerminalGuard {
    active: bool,
}

impl TerminalGuard {
    /// Enters raw mode and the alternate screen, installing a panic hook.
    ///
    /// # Errors
    /// Returns `UiError` if raw mode or alternate screen setup fails.
    pub fn enter() -> Result<Self, UiError> {
        enable_raw_mode()?;
        TERMINAL_IN_RAW_MODE.store(true, Ordering::SeqCst);

        let mut out = stdout();
        execute!(
            out,
            EnterAlternateScreen,
            EnableBracketedPaste,
            EnableFocusChange
        )?;
        out.flush()?;

        // Install panic hook that ensures terminal restoration
        let prev_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |panic_info| {
            restore_terminal();
            prev_hook(panic_info);
        }));

        Ok(Self { active: true })
    }

    /// Disarms the guard without leaving raw mode.
    pub fn disarm(&mut self) {
        self.active = false;
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        if self.active {
            restore_terminal();
        }
    }
}
