//! Input sites.

pub mod reader;
pub use reader::Reader;

use crate::spectrum::{project::Projected, Count};

/// An input site.
///
/// This type results from reader genotypes from a [`Reader`] with its particular configuration.
/// See there for details.
pub enum Site<'a> {
    /// A standard count with no projection.
    Standard(&'a Count),
    /// A projected count.
    Projected(Projected<'a>),
    /// A site with insufficient data.
    InsufficientData,
}
