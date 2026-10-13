//! Tests for terminal state management and panic safety.

use kiln_ui::terminal::restore_terminal;

#[test]
fn test_restore_terminal_idempotent() {
    // Calling restore_terminal when not in raw mode is a safe no-op
    restore_terminal();
    restore_terminal();
    restore_terminal();
}
