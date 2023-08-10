use std::io;

mod builder;
pub use builder::{Builder, BuilderError, Format};

mod genotype;
pub use genotype::{Genotype, GenotypeError, GenotypeResult, GenotypeSkipped};

pub mod sample_map;
use sample_map::Sample;
pub use sample_map::SampleMap;

pub mod bcf;

use crate::{
    spectrum::{
        project::{PartialProjection, Projected},
        Count,
    },
    Scs,
};

use self::sample_map::SampleId;

#[derive(Debug)]
pub enum ReadStatus<T> {
    Read(T),
    Error(io::Error),
    Done,
}

impl<T> ReadStatus<T> {
    pub fn map<U, F>(self, op: F) -> ReadStatus<U>
    where
        F: FnOnce(T) -> U,
    {
        match self {
            ReadStatus::Read(t) => ReadStatus::Read(op(t)),
            ReadStatus::Error(e) => ReadStatus::Error(e),
            ReadStatus::Done => ReadStatus::Done,
        }
    }
}

trait GenotypeReader {
    fn current_contig(&self) -> &str;

    fn current_position(&self) -> usize;

    fn read_genotypes(&mut self) -> ReadStatus<Vec<GenotypeResult>>;

    fn samples(&self) -> &[Sample];
}

pub enum Site<'a> {
    Standard(&'a [usize]),
    Projected(Projected<'a>),
    InsufficientData,
}

pub struct Reader {
    reader: Box<dyn GenotypeReader>,
    sample_map: SampleMap,
    counts: Count,
    totals: Count,
    projection: Option<PartialProjection>,
    skipped_samples: Vec<(SampleId, GenotypeSkipped)>,
}

impl Reader {
    pub fn create_zero_scs(&self) -> Scs {
        let shape = self
            .projection
            .clone()
            .map(|projection| projection.project_to().clone().into_shape())
            .unwrap_or_else(|| self.sample_map.shape());

        Scs::from_zeros(shape)
    }

    pub fn current_contig(&self) -> &str {
        self.reader.current_contig()
    }

    pub fn current_position(&self) -> usize {
        self.reader.current_position()
    }

    pub fn current_skipped_samples(&self) -> impl Iterator<Item = (&Sample, &GenotypeSkipped)> {
        self.skipped_samples
            .iter()
            .map(|(i, s)| (self.sample_map.get_sample(*i).unwrap(), s))
    }

    fn new_unchecked(
        reader: Box<dyn GenotypeReader>,
        sample_map: SampleMap,
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

    pub fn read_site(&mut self) -> ReadStatus<Site<'_>> {
        self.reset();

        let genotypes = match self.reader.read_genotypes() {
            ReadStatus::Read(genotypes) => genotypes,
            ReadStatus::Error(e) => return ReadStatus::Error(e),
            ReadStatus::Done => return ReadStatus::Done,
        };

        for (sample, genotype) in self.reader.samples().iter().zip(genotypes) {
            let Some(population_id) = self.sample_map.get_population_id(sample).map(usize::from) else {
                continue
            };

            match genotype {
                GenotypeResult::Genotype(genotype) => {
                    self.counts[population_id] += genotype as u8 as usize;
                    self.totals[population_id] += 2;
                }
                GenotypeResult::Skipped(skip) => {
                    self.skipped_samples
                        .push((self.sample_map.get_sample_id(sample).unwrap(), skip));
                }
                GenotypeResult::Error(e) => {
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

    pub fn samples(&self) -> &[Sample] {
        self.reader.samples()
    }
}
