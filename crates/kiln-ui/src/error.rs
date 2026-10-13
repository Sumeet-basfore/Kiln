//! Errors for `kiln-ui`.

use thiserror::Error;

/// Error type for terminal UI and rendering operations.
#[derive(Debug, Error)]
pub enum UiError {
    /// An underlying I/O error occurred during terminal operations.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// An error occurred during rendering.
    #[error("Render error: {message}")]
    RenderError {
        /// Explanation of what failed.
        message: String,
    },

    /// An error occurred in terminal state manipulation.
    #[error("Terminal error: {message}")]
    TerminalError {
        /// Explanation of what failed.
        message: String,
    },
}
