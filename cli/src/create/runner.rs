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

pub struct Runner {
    reader: Reader<Box<dyn io::Read>>,
    sample_list: SampleList,
    warnings: Warnings,
    strict: bool,
}

pub struct Reader<R> {
    inner: bcf::Reader<bgzf::Reader<R>>,
    header: vcf::Header,
    string_maps: bcf::header::StringMaps,
    buf: bcf::Record,
}

impl<R> Reader<R>
where
    R: io::Read,
{
    pub fn new(inner: bgzf::Reader<R>) -> io::Result<Self> {
        let mut inner = bcf::Reader::from(inner);

        inner.read_file_format()?;
        let header = inner
            .read_header()?
            .parse()
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let string_maps = bcf::header::StringMaps::from(&header);

        Ok(Self {
            inner,
            header,
            string_maps,
            buf: bcf::Record::default(),
        })
    }

    pub fn contig(&self) -> &str {
        self.string_maps
            .contigs()
            .get_index(self.buf.chromosome_id())
            .unwrap_or("[unknown]")
    }

    pub fn position(&self) -> usize {
        self.buf.position().into()
    }

    pub fn read_genotype_subset(
        &mut self,
        sample_list: &SampleList,
    ) -> io::Result<Option<Result<Genotypes, ParseGenotypesError>>> {
        if self.inner.read_record(&mut self.buf)? > 0 {
            self.buf
                .genotypes()
                .try_into_vcf_record_genotypes(&self.header, self.string_maps.strings())?
                .genotypes()
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
                .map(|genotypes| Some(Genotypes::try_subset_from_iter(genotypes, sample_list)))
        } else {
            Ok(None)
        }
    }
}

impl Runner {
    pub fn new(reader: Reader<Box<dyn io::Read>>, sample_list: SampleList, strict: bool) -> Self {
        Self {
            reader,
            sample_list,
            warnings: Warnings::default(),
            strict,
        }
    }

    pub fn run(&mut self) -> Result<Sfs, Error> {
        let mut sfs = Sfs::from_zeros(self.sample_list.shape());
        let mut allele_counts = AlleleCounts::zeros(sfs.dimensions());

        while let Some(genotypes) = self.reader.read_genotype_subset(&self.sample_list)? {
            match genotypes {
                Ok(genotypes) => {
                    allele_counts.add(&genotypes, &self.sample_list);
                    sfs[&allele_counts] += 1.0;
                    allele_counts.reset();
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
        let reader = Reader::new(bgzf_reader)?;

        let sample_list = if let Some(path) = &args.samples_file {
            SampleList::from_path(path, &reader.header)??
        } else if let Some(names) = &args.samples {
            SampleList::from_names(names, &reader.header)?
        } else {
            SampleList::from_all_samples(&reader.header)
        };

        Ok(Self::new(reader, sample_list, args.strict))
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

    pub fn warn_once<R>(&mut self, reader: &Reader<R>, error: ParseGenotypesError)
    where
        R: io::Read,
    {
        if self.count(error) == 0 {
            let position = reader.position();
            let contig = reader.contig();
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
