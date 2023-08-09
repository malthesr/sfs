use anyhow::{anyhow, Error};

use sfs_core::{
    reader::{ReadStatus, Reader, Site},
    Scs,
};

pub struct Runner {
    reader: Reader,
    strict: bool,
    sites: usize,
    skipped: usize,
}

impl Runner {
    fn handle_skipped_site(&mut self) -> Result<(), Error> {
        let contig = self.reader.current_contig();
        let position = self.reader.current_position();

        if self.strict {
            return Err(anyhow!(
                "Missing or multiallelic genotype at site '{contig}:{position}' in strict mode. \
                Filter BCF or disable strict mode and try again. \
                Increase verbosity for more information."
            ));
        } else {
            if self.skipped == 0 {
                log::info!(
                    "Skipping site '{contig}:{position}' due to too many missing and/or \
                    multiallelic genotypes. By default, this message will be shown only once, \
                    with a summary at the end. Increase verbosity for more information."
                );
            } else {
                log::debug!(
                    "Skipping site '{contig}:{position}' \
                    due to too many missing and/or multiallelic genotypes."
                );
            }

            self.skipped += 1;
        }
        Ok(())
    }

    fn handle_skipped_samples(&self) {
        let contig = self.reader.current_contig();
        let position = self.reader.current_position();

        for (sample, reason) in self
            .reader
            .current_skipped_samples()
            .map(|(sample, skipped_genotype)| (sample.as_ref(), skipped_genotype.reason()))
        {
            log::trace!(
                "Skipping sample '{sample}' at site '{contig}:{position}'. Reason: '{reason}'.",
            )
        }
    }

    pub fn new(reader: Reader, strict: bool) -> Result<Self, Error> {
        Ok(Self {
            reader,
            strict,
            sites: 0,
            skipped: 0,
        })
    }

    pub fn run(&mut self) -> Result<Scs, Error> {
        let mut scs = self.reader.create_zero_scs();

        loop {
            match self.reader.read_site() {
                ReadStatus::Read(Site::Standard(counts)) => {
                    scs[&counts] += 1.0;
                }
                ReadStatus::Read(Site::Projected(projected)) => {
                    projected.add_unchecked(&mut scs);
                }
                ReadStatus::Read(Site::InsufficientData) => {
                    self.handle_skipped_site()?;
                }
                ReadStatus::Error(e) => {
                    return Err(anyhow!(
                        "encountered genotype error at site '{}:{}': {e}",
                        self.reader.current_contig(),
                        self.reader.current_position()
                    ))
                }
                ReadStatus::Done => break,
            }

            self.handle_skipped_samples();

            self.sites += 1;
        }

        self.summarize_skipped();

        Ok(scs)
    }

    fn summarize_skipped(&self) {
        if self.skipped > 0 {
            log::info!(
                "Skipped {skipped}/{total} sites due to missing and/or multiallelic genotypes. \
                Project data (or relax projection) as necessary to keep more sites.",
                skipped = self.skipped,
                total = self.sites,
            );
        }
    }
}
