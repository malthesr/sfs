use std::io;

mod builder;
pub use builder::{Builder, BuilderError, Format};

mod genotype;
pub use genotype::{Genotype, ParseGenotypeError};

pub mod sample_map;
pub use sample_map::SampleMap;
use sample_map::{PopulationId, Sample};

pub mod bcf;

use crate::array::Shape;

trait GenotypeReader {
    fn current_contig(&self) -> &str;

    fn current_position(&self) -> usize;

    fn read_genotypes(&mut self) -> io::Result<Option<Vec<Result<Genotype, ParseGenotypeError>>>>;

    fn samples(&self) -> &[Sample];
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct AlleleCounts(pub Vec<usize>);

impl AlleleCounts {
    fn reset(&mut self) {
        self.0.iter_mut().for_each(|x| *x = 0);
    }
}

impl AsRef<[usize]> for AlleleCounts {
    fn as_ref(&self) -> &[usize] {
        &self.0
    }
}

pub struct Reader {
    reader: Box<dyn GenotypeReader>,
    sample_map: SampleMap,
    buf: AlleleCounts,
}

impl Reader {
    pub fn current_contig(&self) -> &str {
        self.reader.current_contig()
    }

    pub fn current_position(&self) -> usize {
        self.reader.current_position()
    }

    fn new_unchecked(reader: Box<dyn GenotypeReader>, sample_map: SampleMap) -> Self {
        let buf = AlleleCounts(vec![0; sample_map.number_of_populations()]);

        Self {
            reader,
            sample_map,
            buf,
        }
    }

    pub fn read_allele_counts(
        &mut self,
    ) -> io::Result<Option<Result<&AlleleCounts, ParseGenotypeError>>> {
        self.buf.reset();

        let Some(genotypes) = self.reader.read_genotypes()? else {
            return Ok(None)
        };

        for (sample, genotype) in self.reader.samples().iter().zip(genotypes) {
            match (self.sample_map.get(sample), genotype) {
                (Some(population_id), Ok(genotype)) => {
                    self.buf.0[population_id.0] += genotype as u8 as usize;
                }
                (Some(_), Err(e)) => return Ok(Some(Err(e))),
                (None, Ok(_) | Err(_)) => continue,
            }
        }

        Ok(Some(Ok(&self.buf)))
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
