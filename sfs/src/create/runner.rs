use anyhow::Error;

use sfs_core::{
    reader::{ParseGenotypeError, Reader},
    sfs::Sfs,
};

pub struct Runner {
    reader: Reader,
    warnings: Warnings,
    strict: bool,
}

impl Runner {
    pub fn new(reader: Reader, strict: bool) -> Result<Self, Error> {
        Ok(Self {
            reader,
            warnings: Warnings::default(),
            strict,
        })
    }

    pub fn run(&mut self) -> Result<Sfs, Error> {
        let mut sfs = Sfs::from_zeros(self.reader.shape());

        while let Some(allele_counts) = self.reader.read_allele_counts()? {
            match allele_counts {
                Ok(allele_counts) => {
                    sfs[allele_counts] += 1.0;
                }
                Err(error) => {
                    if self.strict {
                        Err(error)?
                    } else {
                        self.warnings.warn_once(&self.reader, error);
                    }
                }
            }
        }

        self.warnings.summarize();

        Ok(sfs)
    }
}

const NUMBER_OF_ERRORS: usize = 4;
const ERROR_VARIANTS: [ParseGenotypeError; NUMBER_OF_ERRORS] = [
    ParseGenotypeError::MissingGenotype,
    ParseGenotypeError::MissingAllele,
    ParseGenotypeError::Multiallelic,
    ParseGenotypeError::NotDiploid,
];

#[derive(Clone, Debug, Default)]
struct Warnings {
    counts: [usize; NUMBER_OF_ERRORS],
}

impl Warnings {
    pub fn count(&self, error: ParseGenotypeError) -> usize {
        self.counts[error as u8 as usize]
    }

    pub fn count_mut(&mut self, error: ParseGenotypeError) -> &mut usize {
        self.counts.get_mut(error as u8 as usize).unwrap()
    }

    pub fn warn_once(&mut self, reader: &Reader, error: ParseGenotypeError) {
        if self.count(error) == 0 {
            let position = reader.current_position();
            let contig = reader.current_contig();
            let reason = error.reason();

            log::warn!(
                "Skipping record at position '{contig}:{position}' due to {reason}. \
                This error will be shown only once, with a summary at the end."
            );
        }

        *self.count_mut(error) += 1;
    }

    pub fn summarize(&self) {
        for error in ERROR_VARIANTS {
            let count = self.count(error);

            if count > 0 {
                let reason = error.reason();

                log::warn!("Skipped {count} records due to {reason}.");
            }
        }
    }
}
