use std::fmt;

pub mod reader;
pub use reader::Reader;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum Genotype {
    Zero = 0,
    One = 1,
    Two = 2,
}

impl Genotype {
    pub fn try_from_raw(raw: usize) -> Option<Self> {
        match raw {
            0 => Some(Self::Zero),
            1 => Some(Self::One),
            2 => Some(Self::Two),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Result {
    Genotype(Genotype),
    Skipped(Skipped),
    Error(Error),
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Skipped {
    Missing,
    Multiallelic,
}

impl Skipped {
    pub fn reason(&self) -> &'static str {
        match self {
            Self::Missing => "missing",
            Self::Multiallelic => "multiallelic",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Error {
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
