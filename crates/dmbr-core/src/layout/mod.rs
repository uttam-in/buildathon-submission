//! Layout engine: capacity modelling, partitioning, font negotiation, balance.

pub mod balance;
pub mod capacity;
pub mod font;
pub mod partitioner;

pub use balance::{balance, BalanceResult};
pub use capacity::{compute_capacity, Capacity};
pub use font::{negotiate_font, FontSpec};
pub use partitioner::partition;
