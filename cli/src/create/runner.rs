use anyhow::{anyhow, Error};

use sfs::{Sfs, Shape};

use super::{
    reader_from_stdin_or_path, Create, GenotypeReader, OrderedSampleList, ParseGenotypeError,
    SampleMap,
};

type Reader = Box<dyn GenotypeReader>;

pub struct Runner {
    reader: Reader,
    sample_list: OrderedSampleList,
    warnings: Warnings,
    strict: bool,
}

impl Runner {
    fn shape(&self) -> Shape {
        let group_id_iter = self.sample_list.iter_groups().filter_map(|&id| id);

        let n = 1 + group_id_iter.clone().max().expect("empty samples list");
        let mut shape = vec![1; n];

        for x in group_id_iter {
            shape[x] += 2;
        }

        Shape(shape)
    }

    pub fn new(
        reader: Box<dyn GenotypeReader>,
        sample_map: SampleMap,
        strict: bool,
    ) -> Result<Self, Error> {
        for name in sample_map.sample_names() {
            if !reader.sample_names().contains(name) {
                let message = format!("sample {name} was not found in input file");
                if strict {
                    return Err(anyhow!(message));
                } else {
                    log::warn!("{message}");
                }
            }
        }

        let sample_list =
            OrderedSampleList::from_map_and_ordered_samples(&sample_map, reader.sample_names());

        Ok(Self {
            reader,
            sample_list,
            warnings: Warnings::default(),
            strict,
        })
    }

    pub fn run(&mut self) -> Result<Sfs, Error> {
        let mut sfs = Sfs::from_zeros(self.shape());
        let mut index = vec![0; sfs.dimensions()];

        let subset_mask = self
            .sample_list
            .iter_groups()
            .map(Option::is_some)
            .collect::<Vec<_>>();
        while let Some(genotypes) = self.reader.read_genotype_subset(&subset_mask)? {
            match genotypes {
                Ok(genotypes) => {
                    genotypes
                        .iter()
                        .zip(
                            self.sample_list
                                .iter_groups()
                                .filter_map(|&group_id| group_id),
                        )
                        .for_each(|(&genotype, group_id)| {
                            index[group_id] += genotype as u8 as usize;
                        });

                    sfs[&index] += 1.0;
                }
                Err(error) => {
                    if self.strict {
                        Err(error)?
                    } else {
                        self.warnings.warn_once(&self.reader, error);
                    }
                }
            }

            index.iter_mut().for_each(|x| *x = 0);
        }

        self.warnings.summarize();

        Ok(sfs)
    }
}

impl TryFrom<&Create> for Runner {
    type Error = Error;

    fn try_from(args: &Create) -> Result<Self, Self::Error> {
        let reader = reader_from_stdin_or_path(args.input.as_ref(), args.threads)?;

        let sample_map = if let Some(path) = &args.samples_file {
            SampleMap::from_path(path)??
        } else if let Some(names) = &args.samples {
            SampleMap::from_names_and_group_names(names.to_vec())?
        } else {
            SampleMap::from_names_in_single_group(reader.sample_names().to_vec())
        };

        Self::new(reader, sample_map, args.strict)
    }
}

#[derive(Clone, Debug, Default)]
struct Warnings {
    counts: [usize; ParseGenotypeError::N],
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
        for error in ParseGenotypeError::VARIANTS {
            let count = self.count(error);

            if count > 0 {
                let reason = error.reason();

                log::warn!("Skipped {count} records due to {reason}.");
            }
        }
    }
}
