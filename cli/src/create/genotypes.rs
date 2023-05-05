use std::{fmt, ops::Deref};

use noodles_vcf::record::genotypes::genotype::field::value::genotype::Genotype as VcfGenotype;

use super::samples::SampleList;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Genotypes(Vec<Genotype>);

impl Genotypes {
    fn try_from_iter<I>(vcf_genotypes: I) -> Result<Self, ParseGenotypesError>
    where
        I: IntoIterator<Item = Option<VcfGenotype>>,
    {
        vcf_genotypes
            .into_iter()
            .map(|genotype| {
                genotype
                    .ok_or(ParseGenotypesError::MissingGenotype)
                    .and_then(Genotype::try_from)
            })
            .collect()
    }

    pub fn try_subset_from_iter<I>(
        vcf_genotypes: I,
        sample_list: &SampleList,
    ) -> Result<Self, ParseGenotypesError>
    where
        I: IntoIterator<Item = Option<VcfGenotype>>,
    {
        Self::try_from_iter(
            vcf_genotypes
                .into_iter()
                .zip(sample_list.iter())
                .filter_map(|(vcf_genotype, group_id)| group_id.map(|_| vcf_genotype)),
        )
    }
}

impl FromIterator<Genotype> for Genotypes {
    fn from_iter<I>(genotypes: I) -> Self
    where
        I: IntoIterator<Item = Genotype>,
    {
        Self(genotypes.into_iter().collect())
    }
}

impl Deref for Genotypes {
    type Target = [Genotype];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum Genotype {
    Zero = 0,
    One = 1,
    Two = 2,
}

impl Genotype {
    fn try_from_raw(raw: usize) -> Option<Self> {
        match raw {
            0 => Some(Self::Zero),
            1 => Some(Self::One),
            2 => Some(Self::Two),
            _ => None,
        }
    }
}

impl TryFrom<VcfGenotype> for Genotype {
    type Error = ParseGenotypesError;

    fn try_from(vcf_genotype: VcfGenotype) -> Result<Self, Self::Error> {
        match &vcf_genotype[..] {
            [a, b] => match (a.position(), b.position()) {
                (Some(a), Some(b)) => {
                    Self::try_from_raw(a + b).ok_or(ParseGenotypesError::Multiallelic)
                }
                (None, None) => Err(ParseGenotypesError::MissingGenotype),
                (Some(_), None) | (None, Some(_)) => Err(ParseGenotypesError::MissingAllele),
            },
            _ => Err(ParseGenotypesError::NotDiploid),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum ParseGenotypesError {
    MissingGenotype = 0,
    MissingAllele = 1,
    Multiallelic = 2,
    NotDiploid = 3,
}

impl ParseGenotypesError {
    pub const N: usize = 4;
    pub const VARIANTS: [ParseGenotypesError; Self::N] = [
        Self::MissingGenotype,
        Self::MissingAllele,
        Self::Multiallelic,
        Self::NotDiploid,
    ];

    pub fn reason(&self) -> &'static str {
        match self {
            Self::MissingGenotype => "missing genotype",
            Self::MissingAllele => "missing genotype allele",
            Self::Multiallelic => "multiallelic genotype",
            Self::NotDiploid => "genotype not diploid",
        }
    }
}

impl fmt::Display for ParseGenotypesError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.reason())
    }
}

impl std::error::Error for ParseGenotypesError {}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct AlleleCounts(pub Vec<usize>);

impl AlleleCounts {
    pub fn add(&mut self, genotypes: &Genotypes, sample_list: &SampleList) {
        genotypes
            .iter()
            .zip(sample_list.iter().filter_map(|&group_id| group_id))
            .for_each(|(&genotype, group_id)| {
                self.0[usize::from(group_id)] += genotype as u8 as usize;
            });
    }

    pub fn reset(&mut self) {
        self.0.iter_mut().for_each(|x| *x = 0);
    }

    pub fn zeros(dimensions: usize) -> Self {
        Self(vec![0; dimensions])
    }
}

impl AsRef<[usize]> for AlleleCounts {
    fn as_ref(&self) -> &[usize] {
        &self.0[..]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::str::FromStr;

    use noodles_vcf as vcf;

    #[test]
    fn test_allele_counts_add() -> Result<(), Box<dyn std::error::Error>> {
        let header = vcf::Header::builder()
            .add_sample_name("sample0")
            .add_sample_name("sample1")
            .add_sample_name("sample2")
            .add_sample_name("sample3")
            .add_sample_name("sample4")
            .build();

        let s = "sample0\tgroup0
sample3\tgroup3
sample1\tgroup2
sample4\tgroup0";

        let sample_list = SampleList::from_str(s, &header)?;

        let genotypes = Genotypes(vec![
            Genotype::Two,
            Genotype::One,
            Genotype::Zero,
            Genotype::One,
        ]);

        let mut allele_counts = AlleleCounts::zeros(sample_list.shape().len());
        allele_counts.add(&genotypes, &sample_list);

        assert_eq!(allele_counts, AlleleCounts(vec![3, 0, 1]));

        Ok(())
    }

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
            Err(ParseGenotypesError::MissingGenotype),
        );

        Ok(())
    }

    #[test]
    fn test_genotype_from_vcf_genotype_missing_allele() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(
            Genotype::try_from(VcfGenotype::from_str("./0")?),
            Err(ParseGenotypesError::MissingAllele),
        );

        assert_eq!(
            Genotype::try_from(VcfGenotype::from_str("1|.")?),
            Err(ParseGenotypesError::MissingAllele),
        );

        Ok(())
    }

    #[test]
    fn test_genotype_from_vcf_genotype_multiallelic() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(
            Genotype::try_from(VcfGenotype::from_str("1/2")?),
            Err(ParseGenotypesError::Multiallelic),
        );

        Ok(())
    }

    #[test]
    fn test_genotype_from_vcf_genotype_not_diploid() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(
            Genotype::try_from(VcfGenotype::from_str("0")?),
            Err(ParseGenotypesError::NotDiploid),
        );

        assert_eq!(
            Genotype::try_from(VcfGenotype::from_str("0/0/0")?),
            Err(ParseGenotypesError::NotDiploid),
        );

        Ok(())
    }
}
