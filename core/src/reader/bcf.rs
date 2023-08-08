use std::{fs::File, io, num::NonZeroUsize, path::Path};

use bcf::lazy::Record as BcfRecord;
use noodles_bcf as bcf;
use noodles_bgzf as bgzf;
use noodles_vcf as vcf;
use noodles_vcf::record::genotypes::sample::value::genotype::Genotype as VcfGenotype;

use super::{
    sample_map::Sample, Genotype, GenotypeError, GenotypeReader, GenotypeResult, GenotypeSkipped,
    ReadStatus,
};

pub struct Reader<R> {
    pub inner: bcf::Reader<bgzf::Reader<R>>,
    pub header: vcf::Header,
    pub string_maps: bcf::header::StringMaps,
    pub samples: Vec<Sample>,
    pub buf: BcfRecord,
}

impl<R> Reader<R>
where
    R: io::Read,
{
    pub fn new(inner: bgzf::Reader<R>) -> io::Result<Self> {
        let mut inner = bcf::Reader::from(inner);

        inner.read_file_format()?;
        let header = inner.read_header()?;
        let string_maps = bcf::header::StringMaps::try_from(&header)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        let samples = header.sample_names().iter().cloned().map(Sample).collect();

        Ok(Self {
            inner,
            header,
            string_maps,
            samples,
            buf: BcfRecord::default(),
        })
    }

    fn read_genotypes(&mut self) -> ReadStatus<Vec<Option<VcfGenotype>>> {
        match self.inner.read_lazy_record(&mut self.buf) {
            Ok(0) => ReadStatus::Done,
            Ok(_) => {
                let result = self
                    .buf
                    .genotypes()
                    .try_into_vcf_record_genotypes(&self.header, self.string_maps.strings())
                    .and_then(|genotypes| {
                        genotypes
                            .genotypes()
                            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
                    });

                match result {
                    Ok(genotypes) => ReadStatus::Read(genotypes),
                    Err(e) => ReadStatus::Error(e),
                }
            }
            Err(e) => ReadStatus::Error(e),
        }
    }
}

impl Reader<File> {
    pub fn from_path<P>(path: P, threads: NonZeroUsize) -> io::Result<Self>
    where
        P: AsRef<Path>,
    {
        let bgzf_reader = bgzf::reader::Builder::default()
            .set_worker_count(threads)
            .build_from_path(path)?;

        Self::new(bgzf_reader)
    }
}

impl Reader<io::StdinLock<'static>> {
    pub fn from_stdin(threads: NonZeroUsize) -> io::Result<Self> {
        let bgzf_reader = bgzf::reader::Builder::default()
            .set_worker_count(threads)
            .build_from_reader(io::stdin().lock());

        Self::new(bgzf_reader)
    }
}

impl<R> GenotypeReader for Reader<R>
where
    R: io::Read,
{
    fn current_contig(&self) -> &str {
        self.string_maps
            .contigs()
            .get_index(self.buf.chromosome_id())
            .unwrap_or("[unknown]")
    }

    fn current_position(&self) -> usize {
        self.buf.position().into()
    }

    fn read_genotypes(&mut self) -> ReadStatus<Vec<GenotypeResult>> {
        self.read_genotypes().map(|vcf_genotypes| {
            vcf_genotypes
                .into_iter()
                .map(GenotypeResult::from)
                .collect()
        })
    }

    fn samples(&self) -> &[Sample] {
        &self.samples
    }
}

impl From<Option<VcfGenotype>> for GenotypeResult {
    fn from(genotype: Option<VcfGenotype>) -> Self {
        match genotype {
            Some(genotype) => match &genotype[..] {
                [a, b] => match (a.position(), b.position()) {
                    (Some(a), Some(b)) => match Genotype::try_from_raw(a + b) {
                        Some(genotype) => GenotypeResult::Genotype(genotype),
                        None => GenotypeResult::Skipped(GenotypeSkipped::Multiallelic),
                    },
                    _ => GenotypeResult::Skipped(GenotypeSkipped::Missing),
                },
                _ => GenotypeResult::Error(GenotypeError::PloidyError),
            },
            None => GenotypeResult::Skipped(GenotypeSkipped::Missing),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::str::FromStr;

    #[test]
    fn test_genotype_from_vcf_genotype() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(
            GenotypeResult::from(Some(VcfGenotype::from_str("0/0")?)),
            GenotypeResult::Genotype(Genotype::Zero)
        );
        assert_eq!(
            GenotypeResult::from(Some(VcfGenotype::from_str("0/1")?)),
            GenotypeResult::Genotype(Genotype::One)
        );
        assert_eq!(
            GenotypeResult::from(Some(VcfGenotype::from_str("1/1")?)),
            GenotypeResult::Genotype(Genotype::Two)
        );

        assert_eq!(
            GenotypeResult::from(Some(VcfGenotype::from_str("0|1")?)),
            GenotypeResult::Genotype(Genotype::One)
        );
        assert_eq!(
            GenotypeResult::from(Some(VcfGenotype::from_str("1|0")?)),
            GenotypeResult::Genotype(Genotype::One)
        );

        Ok(())
    }

    #[test]
    fn test_genotype_from_vcf_genotype_missing_genotype() -> Result<(), Box<dyn std::error::Error>>
    {
        assert_eq!(
            GenotypeResult::from(Some(VcfGenotype::from_str("./.")?)),
            GenotypeResult::Skipped(GenotypeSkipped::Missing),
        );

        Ok(())
    }

    #[test]
    fn test_genotype_from_vcf_genotype_missing_allele() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(
            GenotypeResult::from(Some(VcfGenotype::from_str("./0")?)),
            GenotypeResult::Skipped(GenotypeSkipped::Missing),
        );

        assert_eq!(
            GenotypeResult::from(Some(VcfGenotype::from_str("1|.")?)),
            GenotypeResult::Skipped(GenotypeSkipped::Missing),
        );

        Ok(())
    }

    #[test]
    fn test_genotype_from_vcf_genotype_multiallelic() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(
            GenotypeResult::from(Some(VcfGenotype::from_str("1/2")?)),
            GenotypeResult::Skipped(GenotypeSkipped::Multiallelic),
        );

        Ok(())
    }

    #[test]
    fn test_genotype_from_vcf_genotype_not_diploid() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(
            GenotypeResult::from(Some(VcfGenotype::from_str("0")?)),
            GenotypeResult::Error(GenotypeError::PloidyError),
        );

        assert_eq!(
            GenotypeResult::from(Some(VcfGenotype::from_str("0/0/0")?)),
            GenotypeResult::Error(GenotypeError::PloidyError),
        );

        Ok(())
    }
}
