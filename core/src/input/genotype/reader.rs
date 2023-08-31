//! Genotype reading.

use crate::input::{ReadStatus, Sample};

pub mod builder;
pub use builder::Builder;

mod bcf;
mod vcf;

use super::Result;

/// An alias for a trait-object [`Reader`].
pub type DynReader = Box<dyn Reader>;

/// A type capable of reading genotypes for creating spectrum.
pub trait Reader {
    /// Returns the current contig of the reader.
    fn current_contig(&self) -> &str;

    /// Returns the current position of the reader within its current contig.
    fn current_position(&self) -> usize;

    /// Returns the genotypes at the next position in the reader.
    fn read_genotypes(&mut self) -> ReadStatus<Vec<Result>>;

    /// Returns the samples defined by the reader.
    fn samples(&self) -> &[Sample];
}
