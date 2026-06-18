//! Layout engine: capacity modelling, partitioning, font negotiation, balance.

pub mod balance;
pub mod capacity;
pub mod font;
pub mod paginate;
pub mod partitioner;

pub use balance::{balance, BalanceResult};
pub use capacity::{compute_capacity, Capacity};
pub use font::{negotiate_font, FontSpec};
pub use paginate::paginate;
pub use partitioner::partition;
