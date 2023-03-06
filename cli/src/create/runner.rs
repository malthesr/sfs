use std::{fs::File, io};

use anyhow::{Context, Error};

use clap::CommandFactory;
use noodles_bcf as bcf;
use noodles_bgzf as bgzf;
use noodles_vcf as vcf;

use sfs::Sfs;

use super::{
    genotypes::{AlleleCounts, Genotypes, ParseGenotypesError},
    samples::SampleList,
    Create,
};

type Reader = bcf::Reader<bgzf::Reader<Box<dyn io::Read>>>;

pub struct Runner {
    reader: Reader,
    header: vcf::Header,
    string_maps: bcf::header::StringMaps,
    sample_list: SampleList,
    warnings: Warnings,
    strict: bool,
}

impl Runner {
    pub fn new(reader: Reader, header: vcf::Header, sample_list: SampleList, strict: bool) -> Self {
        let string_maps = bcf::header::StringMaps::from(&header);

        Self {
            reader,
            header,
            string_maps,
            sample_list,
            warnings: Warnings::default(),
            strict,
        }
    }

    pub fn run(&mut self) -> Result<Sfs, Error> {
        let mut sfs = Sfs::zeros(self.sample_list.shape());
        let mut allele_counts = AlleleCounts::zeros(sfs.dimensions());

        let mut record = bcf::Record::default();

        while self.reader.read_record(&mut record)? > 0 {
            let genotypes = record
                .genotypes()
                .try_into_vcf_record_genotypes(&self.header, self.string_maps.strings())?
                .genotypes()?;

            let genotypes = match Genotypes::try_subset_from_iter(genotypes, &self.sample_list) {
                Ok(genotypes) => genotypes,
                Err(error) => {
                    if self.strict {
                        Err(error)?
                    } else {
                        self.warnings.warn_once(&record, &self.string_maps, error);
                        continue;
                    }
                }
            };

            allele_counts.add(&genotypes, &self.sample_list);
            sfs[&allele_counts] += 1.0;
            allele_counts.reset();
        }

        self.warnings.summarize();

        Ok(sfs)
    }
}

impl TryFrom<&Create> for Runner {
    type Error = Error;

    fn try_from(args: &Create) -> Result<Self, Self::Error> {
        let inner: Box<dyn io::Read> = if let Some(path) = &args.input {
            Box::new(File::open(path).with_context(|| {
                format!("Failed to open BCF from provided path '{}'", path.display())
            })?)
        } else if atty::isnt(atty::Stream::Stdin) {
            Box::new(io::stdin().lock())
        } else {
            Err(
                clap::Error::new(clap::error::ErrorKind::MissingRequiredArgument)
                    .with_cmd(&Create::command()),
            )?
        };

        let bgzf_reader = bgzf::reader::Builder::default()
            .set_worker_count(args.threads)
            .build_from_reader(inner);
        let mut reader = bcf::Reader::from(bgzf_reader);
        reader.read_file_format()?;
        let header = reader.read_header()?.parse()?;

        let sample_list = if let Some(path) = &args.samples_file {
            SampleList::from_path(path, &header)??
        } else if let Some(names) = &args.samples {
            SampleList::from_names(names, &header)?
        } else {
            SampleList::from_all_samples(&header)
        };

        Ok(Self::new(reader, header, sample_list, args.strict))
    }
}

#[derive(Clone, Debug, Default)]
struct Warnings {
    counts: [usize; ParseGenotypesError::N],
}

impl Warnings {
    pub fn count(&self, error: ParseGenotypesError) -> usize {
        self.counts[error as u8 as usize]
    }

    pub fn count_mut(&mut self, error: ParseGenotypesError) -> &mut usize {
        self.counts.get_mut(error as u8 as usize).unwrap()
    }

    pub fn warn_once(
        &mut self,
        record: &bcf::Record,
        string_maps: &bcf::header::StringMaps,
        error: ParseGenotypesError,
    ) {
        if self.count(error) == 0 {
            let position = record.position();
            let contig = string_maps
                .contigs()
                .get_index(record.chromosome_id())
                .unwrap_or("[unknown]");
            let reason = error.reason();

            log::warn!(
                "Skipping record at position '{contig}:{position}' due to {reason}. \
                This error will be shown only once, with a summary at the end."
            );
        }

        *self.count_mut(error) += 1;
    }

    pub fn summarize(&self) {
        for error in ParseGenotypesError::VARIANTS {
            let count = self.count(error);

            if count > 0 {
                let reason = error.reason();

                log::warn!("Skipped {count} records due to {reason}.");
            }
        }
    }
}
