//! Site reader.

use std::io;

pub mod builder;
pub use builder::Builder;

use crate::{
    input::{genotype, sample, ReadStatus, Sample},
    spectrum::{project::PartialProjection, Count},
    Scs,
};

use super::Site;

/// A site reader.
pub struct Reader {
    reader: Box<dyn genotype::Reader>,
    sample_map: sample::Map,
    counts: Count,
    totals: Count,
    projection: Option<PartialProjection>,
    skipped_samples: Vec<(sample::Id, genotype::Skipped)>,
}

impl Reader {
    /// Returns a spectrum filled with zeros corresponding to the shape defined by the reader
    /// configuration.
    pub fn create_zero_scs(&self) -> Scs {
        let shape = self
            .projection
            .clone()
            .map(|projection| projection.project_to().clone().into_shape())
            .unwrap_or_else(|| self.sample_map.shape());

        Scs::from_zeros(shape)
    }

    /// Returns the current contig of the reader.
    pub fn current_contig(&self) -> &str {
        self.reader.current_contig()
    }

    /// Returns the current position of the reader within its current contig.
    pub fn current_position(&self) -> usize {
        self.reader.current_position()
    }

    /// Returns an iterator over the currently skipped genotypes in the reader, with their
    /// associated samples.
    pub fn current_skipped_samples(&self) -> impl Iterator<Item = (&Sample, &genotype::Skipped)> {
        self.skipped_samples
            .iter()
            .map(|(i, s)| (self.sample_map.get_sample(*i).unwrap(), s))
    }

    fn new_unchecked(
        reader: Box<dyn genotype::Reader>,
        sample_map: sample::Map,
        projection: Option<PartialProjection>,
    ) -> Self {
        let dimensions = sample_map.number_of_populations();

        Self {
            reader,
            sample_map,
            projection,
            counts: Count::from_zeros(dimensions),
            totals: Count::from_zeros(dimensions),
            skipped_samples: Vec::new(),
        }
    }

    /// Reads the next site in the reader.
    pub fn read_site(&mut self) -> ReadStatus<Site<'_>> {
        self.reset();

        let genotypes = match self.reader.read_genotypes() {
            ReadStatus::Read(genotypes) => genotypes,
            ReadStatus::Error(e) => return ReadStatus::Error(e),
            ReadStatus::Done => return ReadStatus::Done,
        };

        for (sample, genotype) in self.reader.samples().iter().zip(genotypes) {
            let Some(population_id) = self.sample_map.get_population_id(sample).map(usize::from)
            else {
                continue;
            };

            match genotype {
                genotype::Result::Genotype(genotype) => {
                    self.counts[population_id] += genotype as u8 as usize;
                    self.totals[population_id] += 2;
                }
                genotype::Result::Skipped(skip) => {
                    self.skipped_samples
                        .push((self.sample_map.get_sample_id(sample).unwrap(), skip));
                }
                genotype::Result::Error(e) => {
                    return ReadStatus::Error(io::Error::new(io::ErrorKind::InvalidData, e));
                }
            }
        }

        let site = if let Some(projection) = self.projection.as_mut() {
            let (exact, projectable) = self.totals.iter().zip(projection.project_to().iter()).fold(
                (true, true),
                |(exact, projectable), (&total, &to)| {
                    (exact && total == to, projectable && total >= to)
                },
            );

            if exact {
                Site::Standard(&self.counts)
            } else if projectable {
                Site::Projected(projection.project_unchecked(&self.totals, &self.counts))
            } else {
                Site::InsufficientData
            }
        } else if self.skipped_samples.is_empty() {
            Site::Standard(&self.counts)
        } else {
            Site::InsufficientData
        };

        ReadStatus::Read(site)
    }

    fn reset(&mut self) {
        self.counts.set_zero();
        self.totals.set_zero();
        self.skipped_samples.clear();
    }

    /// Returns the samples defined by the reader.
    pub fn samples(&self) -> &[Sample] {
        self.reader.samples()
    }
}
