use std::io;

use noodles_bcf as bcf;

use noodles_bgzf as bgzf;

use noodles_vcf as vcf;
use vcf::record::genotypes::genotype::field::value::genotype::Genotype as VcfGenotype;

use super::{Genotype, GenotypeReader, ParseGenotypeError};

pub struct Reader<R> {
    pub inner: bcf::Reader<bgzf::Reader<R>>,
    pub header: vcf::Header,
    pub string_maps: bcf::header::StringMaps,
    pub sample_names: Vec<String>,
    pub buf: bcf::Record,
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

        let sample_names = header.sample_names().iter().cloned().collect();

        Ok(Self {
            inner,
            header,
            string_maps,
            sample_names,
            buf: bcf::Record::default(),
        })
    }

    fn read_genotypes(&mut self) -> io::Result<Option<Vec<Option<VcfGenotype>>>> {
        if self.inner.read_record(&mut self.buf)? > 0 {
            let vcf_genotypes = self
                .buf
                .genotypes()
                .try_into_vcf_record_genotypes(&self.header, self.string_maps.strings())?
                .genotypes()
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

            Ok(Some(vcf_genotypes))
        } else {
            Ok(None)
        }
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

    fn read_genotype_subset(
        &mut self,
        subset_mask: &[bool],
    ) -> io::Result<Option<Result<Vec<Genotype>, ParseGenotypeError>>> {
        match self.read_genotypes()? {
            Some(vcf_genotypes) => {
                let genotypes = vcf_genotypes
                    .into_iter()
                    .zip(subset_mask)
                    .filter_map(|(vcf_genotype, keep)| keep.then_some(vcf_genotype))
                    .map(|vcf_genotype| {
                        vcf_genotype
                            .ok_or(ParseGenotypeError::MissingGenotype)
                            .and_then(Genotype::try_from)
                    })
                    .collect::<Result<Vec<_>, _>>();

                Ok(Some(genotypes))
            }
            None => Ok(None),
        }
    }

    fn sample_names(&self) -> &[String] {
        &self.sample_names
    }
}

impl TryFrom<VcfGenotype> for Genotype {
    type Error = ParseGenotypeError;

    fn try_from(vcf_genotype: VcfGenotype) -> Result<Self, Self::Error> {
        match &vcf_genotype[..] {
            [a, b] => match (a.position(), b.position()) {
                (Some(a), Some(b)) => {
                    Self::try_from_raw(a + b).ok_or(ParseGenotypeError::Multiallelic)
                }
                (None, None) => Err(ParseGenotypeError::MissingGenotype),
                (Some(_), None) | (None, Some(_)) => Err(ParseGenotypeError::MissingAllele),
            },
            _ => Err(ParseGenotypeError::NotDiploid),
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
            Genotype::try_from(VcfGenotype::from_str("0/0")?),
            Ok(Genotype::Zero)
        );
        assert_eq!(
            Genotype::try_from(VcfGenotype::from_str("0/1")?),
            Ok(Genotype::One)
        );
        assert_eq!(
            Genotype::try_from(VcfGenotype::from_str("1/1")?),
            Ok(Genotype::Two)
        );

        assert_eq!(
            Genotype::try_from(VcfGenotype::from_str("0|1")?),
            Ok(Genotype::One)
        );
        assert_eq!(
            Genotype::try_from(VcfGenotype::from_str("1|0")?),
            Ok(Genotype::One)
        );

        Ok(())
    }

    #[test]
    fn test_genotype_from_vcf_genotype_missing_genotype() -> Result<(), Box<dyn std::error::Error>>
    {
        assert_eq!(
            Genotype::try_from(VcfGenotype::from_str("./.")?),
            Err(ParseGenotypeError::MissingGenotype),
        );

        Ok(())
    }

    #[test]
    fn test_genotype_from_vcf_genotype_missing_allele() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(
            Genotype::try_from(VcfGenotype::from_str("./0")?),
            Err(ParseGenotypeError::MissingAllele),
        );

        assert_eq!(
            Genotype::try_from(VcfGenotype::from_str("1|.")?),
            Err(ParseGenotypeError::MissingAllele),
        );

        Ok(())
    }

    #[test]
    fn test_genotype_from_vcf_genotype_multiallelic() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(
            Genotype::try_from(VcfGenotype::from_str("1/2")?),
            Err(ParseGenotypeError::Multiallelic),
        );

        Ok(())
    }

    #[test]
    fn test_genotype_from_vcf_genotype_not_diploid() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(
            Genotype::try_from(VcfGenotype::from_str("0")?),
            Err(ParseGenotypeError::NotDiploid),
        );

        assert_eq!(
            Genotype::try_from(VcfGenotype::from_str("0/0/0")?),
            Err(ParseGenotypeError::NotDiploid),
        );

        Ok(())
    }
}
