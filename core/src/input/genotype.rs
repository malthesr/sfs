//! Diploid, diallelic genotype.

use std::fmt;

pub mod reader;
pub use reader::Reader;

/// A diploid, diallelic genotype, coded as the number of minor/alternative/derived alleles.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum Genotype {
    /// Zero alleles.
    Zero = 0,
    /// One alleles.
    One = 1,
    /// Two alleles.
    Two = 2,
}

impl Genotype {
    /// Returns a genotype its raw representation if possible, otherwise `None`.
    pub fn try_from_raw(raw: usize) -> Option<Self> {
        match raw {
            0 => Some(Self::Zero),
            1 => Some(Self::One),
            2 => Some(Self::Two),
            _ => None,
        }
    }
}

/// The result of trying to read a genotype.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Result {
    /// A genotype that was succesfully read and parsed.
    Genotype(Genotype),
    /// A genotype that was read, but skipped (e.g. multiallelic, missing)
    Skipped(Skipped),
    /// An error.
    Error(Error),
}

/// A reason for skipping a genotype.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Skipped {
    /// Genotype was missing.
    Missing,
    /// Genotype was multiallelic.
    Multiallelic,
}

impl Skipped {
    /// Returns a string representation for having skipped the genotype.
    pub fn reason(&self) -> &'static str {
        match self {
            Self::Missing => "missing",
            Self::Multiallelic => "multiallelic",
        }
    }
}

/// An error associated with parsing a genotype.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Error {
    /// Genotype not diploid.
    PloidyError,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::PloidyError => f.write_str("genotype not diploid"),
        }
    }
}

impl std::error::Error for Error {}
