//! Library surface for the challenge-format adapter.
//!
//! Exposes the challenge input models ([`challenge`]) and the adapter
//! ([`adapt`]) that maps them into `dmbr-core`'s normalized schema, so other
//! crates (e.g. the web servers) can reuse the exact same parsing and
//! state-resolution logic the `dmbr-convert` binary uses.

pub mod adapt;
pub mod challenge;
