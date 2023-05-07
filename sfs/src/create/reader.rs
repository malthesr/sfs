use std::{fmt, fs::File, io, num::NonZeroUsize, path::Path};

use anyhow::{Context, Error};

use noodles_bgzf as bgzf;

mod vcf;
pub use vcf::Reader as VcfReader;

pub fn reader_from_stdin_or_path<P>(
    path: Option<P>,
    threads: NonZeroUsize,
) -> Result<Box<dyn GenotypeReader>, Error>
where
    P: AsRef<Path>,
{
    let bgzf_builder = bgzf::reader::Builder::default().set_worker_count(threads);

    let reader: Box<dyn GenotypeReader> = if let Some(path) = path.as_ref() {
        let file = File::open(path).with_context(|| {
            format!(
                "Failed to open BCF from provided path '{}'",
                path.as_ref().display()
            )
        })?;

        let inner = bgzf_builder.build_from_reader(file);

        VcfReader::new(inner).map(Box::new)?
    } else if atty::isnt(atty::Stream::Stdin) {
        let inner = bgzf_builder.build_from_reader(io::stdin().lock());

        VcfReader::new(inner).map(Box::new)?
    } else {
        Err(clap::Error::new(
            clap::error::ErrorKind::MissingRequiredArgument,
        ))?
    };

    Ok(reader)
}

pub trait GenotypeReader {
    fn current_contig(&self) -> &str;

    fn current_position(&self) -> usize;

    fn read_genotype_subset(
        &mut self,
        subset_mask: &[bool],
    ) -> io::Result<Option<Result<Vec<Genotype>, ParseGenotypeError>>>;

    fn sample_names(&self) -> &[String];
}

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
    pub const N: usize = 4;

    pub const VARIANTS: [ParseGenotypeError; Self::N] = [
        Self::MissingGenotype,
        Self::MissingAllele,
        Self::Multiallelic,
        Self::NotDiploid,
    ];

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
