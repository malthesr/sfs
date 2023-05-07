use std::fmt;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum ParseGenotypeError {
    MissingGenotype = 0,
    MissingAllele = 1,
    Multiallelic = 2,
    NotDiploid = 3,
}

impl ParseGenotypeError {
    pub fn reason(&self) -> &'static str {
        match self {
            Self::MissingGenotype => "missing genotype",
            Self::MissingAllele => "missing genotype allele",
            Self::Multiallelic => "multiallelic genotype",
            Self::NotDiploid => "genotype not diploid",
        }
    }
}

impl fmt::Display for ParseGenotypeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.reason())
    }
}

impl std::error::Error for ParseGenotypeError {}
