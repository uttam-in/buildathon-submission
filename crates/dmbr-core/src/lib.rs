//! # dmbr-core
//!
//! Core library for the Digital Menu Board Layout Renderer.
//!
//! The crate is organised into a small set of stages that run in sequence:
//!
//! 1. [`models`] — serde data models for the JSON inputs and output.
//! 2. [`pipeline`] — meal-period detection, filtering, and canonical ordering.
//! 3. [`layout`] — capacity modelling, multi-screen partitioning, font
//!    negotiation, and balance scoring.
//! 4. [`renderer`] — self-contained HTML/CSS generation per screen.
//! 5. [`hash`] — deterministic SHA-256 hashing of the rendered output.
//!
//! The top-level [`render`] function ties these together into a single
//! deterministic call producing a [`models::LayoutOutput`].

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod error;
pub mod hash;
pub mod layout;
pub mod models;
pub mod pipeline;
pub mod renderer;

pub use error::{RenderError, Result};
pub use models::{DayState, FullMenu, LayoutOutput, ScreenConfig};
