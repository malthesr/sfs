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
    skips: Vec<(usize, GenotypeSkipped)>,
}

impl Reader {
    pub fn create_zero_scs(&self) -> Scs {
        let shape = self
            .projection
            .clone()
            .map(|proj| proj.to_count().clone().into_shape())
            .unwrap_or_else(|| self.sample_map.shape().clone());

        Scs::from_zeros(shape)
    }

    pub fn current_contig(&self) -> &str {
        self.reader.current_contig()
    }

    pub fn current_position(&self) -> usize {
        self.reader.current_position()
    }

    pub fn current_skips(&self) -> impl Iterator<Item = (&Sample, &GenotypeSkipped)> {
        self.skips
            .iter()
            .map(|(i, skip)| (self.sample_map.by_index(*i).unwrap(), skip))
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
            skips: Vec::new(),
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
            let Some(population_id) = self.sample_map.get(sample).map(usize::from) else {
                continue
            };

            match genotype {
                GenotypeResult::Genotype(genotype) => {
                    self.counts[population_id] += genotype as u8 as usize;
                    self.totals[population_id] += 2;
                }
                GenotypeResult::Skipped(skip) => {
                    self.skips
                        .push((self.sample_map.index_of(sample).unwrap(), skip));
                }
                GenotypeResult::Error(e) => {
                    return ReadStatus::Error(io::Error::new(io::ErrorKind::InvalidData, e));
                }
            }
        }

        let site = if let Some(projection) = self.projection.as_ref() {
            let (exact, projectable) = self.totals.iter().zip(projection.to_count().iter()).fold(
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
        } else {
            if self.skips.is_empty() {
                Site::Standard(&self.counts)
            } else {
                Site::InsufficientData
            }
        };

        ReadStatus::Read(site)
    }

    fn reset(&mut self) {
        self.counts.set_zero();
        self.totals.set_zero();
        self.skips.clear();
    }

    pub fn samples(&self) -> &[Sample] {
        self.reader.samples()
    }
}
