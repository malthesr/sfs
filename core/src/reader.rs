use std::io;

mod builder;
pub use builder::{Builder, BuilderError, Format};

mod genotype;
pub use genotype::{Genotype, GenotypeError, GenotypeResult, GenotypeSkipped};

pub mod sample_map;
pub use sample_map::SampleMap;
use sample_map::{PopulationId, Sample};

pub mod bcf;

use crate::{
    array::Shape,
    spectrum::project::{ProjectError, Projected, Projection},
};

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
    counts: Vec<usize>,
    projection: Option<Projection>,
    skips: Vec<(usize, GenotypeSkipped)>,
}

impl Reader {
    pub fn current_skips(&self) -> impl Iterator<Item = (&Sample, &GenotypeSkipped)> {
        self.skips
            .iter()
            .map(|(i, skip)| (self.sample_map.by_index(*i).unwrap(), skip))
    }

    fn reset(&mut self) {
        self.counts.iter_mut().for_each(|x| *x = 0);
        self.skips.clear();
    }

    pub fn current_contig(&self) -> &str {
        self.reader.current_contig()
    }

    pub fn current_position(&self) -> usize {
        self.reader.current_position()
    }

    fn new_unchecked(reader: Box<dyn GenotypeReader>, sample_map: SampleMap) -> Self {
        let site = vec![0; sample_map.number_of_populations()];

        Self {
            reader,
            sample_map,
            projection: None,
            counts: site,
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
            let Some(population_id) = self.sample_map.get(sample) else {
                continue
            };

            match genotype {
                GenotypeResult::Genotype(genotype) => {
                    self.counts[population_id.0] += genotype as u8 as usize;
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
            match projection.project(&self.counts) {
                Ok(projected) => Site::Projected(projected),
                Err(ProjectError::InvalidProjection { .. }) => Site::InsufficientData,
                Err(ProjectError::UnequalDimensions { .. }) => unreachable!(),
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

    pub fn samples(&self) -> &[Sample] {
        self.reader.samples()
    }

    pub fn shape(&self) -> Shape {
        let population_sizes = self.sample_map.population_sizes();

        Shape(
            (0..population_sizes.len())
                .map(|id| 1 + 2 * population_sizes.get(&PopulationId(id)).unwrap())
                .collect(),
        )
    }
}
