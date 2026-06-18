//! Error types for the rendering pipeline.

use thiserror::Error;

/// Errors that can occur while validating inputs, laying out, or rendering.
#[derive(Debug, Error)]
pub enum RenderError {
    /// An input failed validation (bad JSON values, unsupported configuration).
    #[error("invalid input: {0}")]
    InvalidInput(String),

    /// The layout engine could not place content (e.g. impossible geometry).
    #[error("layout error: {0}")]
    LayoutError(String),

    /// HTML generation or hashing failed.
    #[error("render error: {0}")]
    RenderError(String),
}

/// Convenience alias for results returned by the library.
pub type Result<T> = std::result::Result<T, RenderError>;
