use std::io;

use noodles_vcf as vcf;
use vcf::record::{
    genotypes::sample::value::genotype::Genotype as VcfGenotype, Record as VcfRecord,
};

use super::{
    Genotype, GenotypeError, GenotypeReader, GenotypeResult, GenotypeSkipped, ReadStatus, Sample,
};

pub struct Reader<R> {
    pub inner: vcf::Reader<R>,
    pub header: vcf::Header,
    pub samples: Vec<Sample>,
    pub buf: VcfRecord,
}

impl<R> Reader<R>
where
    R: io::BufRead,
{
    pub fn new(inner: R) -> io::Result<Self> {
        let mut inner = vcf::Reader::new(inner);

        let header = inner.read_header()?;
        let samples = header
            .sample_names()
            .iter()
            .cloned()
            .map(Sample::from)
            .collect();

        Ok(Self {
            inner,
            header,
            samples,
            buf: VcfRecord::default(),
        })
    }

    fn read_genotypes(&mut self) -> ReadStatus<Vec<Option<VcfGenotype>>> {
        match self.inner.read_record(&self.header, &mut self.buf) {
            Ok(0) => ReadStatus::Done,
            Ok(_) => {
                let result = self
                    .buf
                    .genotypes()
                    .genotypes()
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e));

                match result {
                    Ok(genotypes) => ReadStatus::Read(genotypes),
                    Err(e) => ReadStatus::Error(e),
                }
            }
            Err(e) => ReadStatus::Error(e),
        }
    }
}

impl<R> GenotypeReader for Reader<R>
where
    R: io::BufRead,
{
    fn current_contig(&self) -> &str {
        match self.buf.chromosome() {
            vcf::record::Chromosome::Name(s) | vcf::record::Chromosome::Symbol(s) => s,
        }
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
