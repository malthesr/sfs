use std::{
    fmt,
    fs::File,
    io::{self, Read},
    path::Path,
};

use indexmap::IndexSet;

use noodles_vcf as vcf;

use sfs::Shape;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SampleList {
    inner: Vec<Option<GroupId>>,
}

impl SampleList {
    pub fn from_all_samples(header: &vcf::Header) -> Self {
        let inner = vec![Some(GroupId(0)); header.sample_names().len()];

        Self::new(inner)
    }

    pub fn from_path<P>(
        path: P,
        header: &vcf::Header,
    ) -> io::Result<Result<Self, ParseSamplesError>>
    where
        P: AsRef<Path>,
    {
        let mut file = File::open(path)?;
        let mut s = String::new();
        let _ = file.read_to_string(&mut s)?;

        Ok(Self::from_str(&s, header))
    }

    fn from_samples(samples: &[Sample], header: &vcf::Header) -> Result<Self, ParseSamplesError> {
        if samples.is_empty() {
            return Err(ParseSamplesError::Empty);
        }

        let mut inner = vec![None; header.sample_names().len()];

        for sample in samples {
            inner[usize::from(sample.header_position)] = Some(sample.group_id);
        }

        Ok(Self::new(inner))
    }

    pub fn from_names(
        names: &[(String, Option<String>)],
        header: &vcf::Header,
    ) -> Result<Self, ParseSamplesError> {
        let header_positions = HeaderPositions::from(header);
        let mut group_ids = GroupIds::empty();

        let samples = names
            .iter()
            .map(|(sample_name, group_name)| {
                Sample::from_names(
                    sample_name,
                    group_name.as_deref(),
                    &header_positions,
                    &mut group_ids,
                )
            })
            .collect::<Result<Vec<_>, _>>()?;

        Self::from_samples(&samples, header)
    }

    pub fn from_str(s: &str, header: &vcf::Header) -> Result<Self, ParseSamplesError> {
        let header_positions = HeaderPositions::from(header);
        let mut group_ids = GroupIds::empty();

        let samples = s
            .lines()
            .map(|line| Sample::from_str(line, &header_positions, &mut group_ids))
            .collect::<Result<Vec<_>, _>>()?;

        Self::from_samples(&samples, header)
    }

    pub fn iter(&self) -> std::slice::Iter<Option<GroupId>> {
        self.inner.iter()
    }

    fn new(inner: Vec<Option<GroupId>>) -> Self {
        Self { inner }
    }

    pub fn shape(&self) -> Shape {
        let group_id_iter = self.iter().filter_map(|id| id.map(usize::from));

        let n = 1 + group_id_iter.clone().max().expect("empty samples list");
        let mut shape = vec![1; n];

        for x in group_id_iter {
            shape[x] += 2;
        }

        Shape(shape)
    }
}

impl FromIterator<Option<GroupId>> for SampleList {
    fn from_iter<I>(group_ids: I) -> Self
    where
        I: IntoIterator<Item = Option<GroupId>>,
    {
        Self::new(group_ids.into_iter().collect())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Sample {
    header_position: HeaderPosition,
    group_id: GroupId,
}

impl Sample {
    fn from_names(
        sample_name: &str,
        group_name: Option<&str>,
        header_positions: &HeaderPositions,
        group_ids: &mut GroupIds,
    ) -> Result<Self, ParseSamplesError> {
        let group_name = group_name.unwrap_or_default();

        let header_position = header_positions
            .get_position_of(sample_name)
            .ok_or_else(|| ParseSamplesError::unknown_sample(sample_name.to_string()))?;

        let group_id = group_ids
            .get_id_of(group_name)
            .unwrap_or_else(|| group_ids.create_group(group_name));

        Ok(Self::new(header_position, group_id))
    }

    fn from_str(
        s: &str,
        header_positions: &HeaderPositions,
        group_ids: &mut GroupIds,
    ) -> Result<Self, ParseSamplesError> {
        let (sample_name, group_name) = match s.split_once('\t') {
            Some((sample_name, group_name)) => (sample_name, Some(group_name)),
            None => (s, None),
        };

        Self::from_names(sample_name, group_name, header_positions, group_ids)
    }

    fn new(header_position: HeaderPosition, group_id: GroupId) -> Self {
        Self {
            header_position,
            group_id,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ParseSamplesError {
    UnknownSample { sample_name: String },
    Empty,
}

impl ParseSamplesError {
    pub fn unknown_sample(sample_name: String) -> Self {
        Self::UnknownSample { sample_name }
    }
}

impl fmt::Display for ParseSamplesError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseSamplesError::UnknownSample { sample_name } => {
                write!(f, "sample name '{sample_name}' not found in input header",)
            }
            ParseSamplesError::Empty => f.write_str("empty samples list"),
        }
    }
}

impl std::error::Error for ParseSamplesError {}

#[derive(Clone, Debug, Eq, PartialEq)]
struct HeaderPositions(IndexSet<String>);

impl HeaderPositions {
    pub fn get_position_of(&self, sample_name: &str) -> Option<HeaderPosition> {
        self.0.get_index_of(sample_name).map(HeaderPosition)
    }
}

impl From<&vcf::Header> for HeaderPositions {
    fn from(header: &vcf::Header) -> Self {
        Self(header.sample_names().clone())
    }
}

impl<S> FromIterator<S> for HeaderPositions
where
    S: ToString,
{
    fn from_iter<I>(sample_names: I) -> Self
    where
        I: IntoIterator<Item = S>,
    {
        Self(sample_names.into_iter().map(|s| s.to_string()).collect())
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
struct HeaderPosition(usize);

impl From<HeaderPosition> for usize {
    fn from(header_position: HeaderPosition) -> Self {
        header_position.0
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct GroupIds(IndexSet<String>);

impl GroupIds {
    pub fn create_group<S>(&mut self, group_name: S) -> GroupId
    where
        S: ToString,
    {
        GroupId(self.0.insert_full(group_name.to_string()).0)
    }

    pub fn empty() -> Self {
        Self(IndexSet::new())
    }

    pub fn get_id_of(&self, group_name: &str) -> Option<GroupId> {
        self.0.get_index_of(group_name).map(GroupId)
    }
}

impl<S> FromIterator<S> for GroupIds
where
    S: ToString,
{
    fn from_iter<I>(group_names: I) -> Self
    where
        I: IntoIterator<Item = S>,
    {
        Self(group_names.into_iter().map(|s| s.to_string()).collect())
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct GroupId(usize);

impl From<GroupId> for usize {
    fn from(group_id: GroupId) -> Self {
        group_id.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_header_positions_from_vcf_header() {
        let header = vcf::Header::builder()
            .add_sample_name("sample0")
            .add_sample_name("sample1")
            .add_sample_name("sample2")
            .build();

        let expected = HeaderPositions::from_iter(["sample0", "sample1", "sample2"]);

        assert_eq!(HeaderPositions::from(&header), expected);
    }

    #[test]
    fn test_sample_from_str() -> Result<(), ParseSamplesError> {
        let header_positions = HeaderPositions::from_iter(["sample0"]);
        let mut group_ids = GroupIds::empty();

        let sample = Sample::from_str("sample0\tgroup0", &header_positions, &mut group_ids)?;

        assert_eq!(sample, Sample::new(HeaderPosition(0), GroupId(0)));
        assert_eq!(group_ids, GroupIds::from_iter(["group0"]));

        Ok(())
    }

    #[test]
    fn test_sample_from_str_default_group() -> Result<(), ParseSamplesError> {
        let header_positions = HeaderPositions::from_iter(["sample0"]);
        let mut group_ids = GroupIds::from_iter(["group0"]);

        let sample = Sample::from_str("sample0", &header_positions, &mut group_ids)?;

        assert_eq!(sample, Sample::new(HeaderPosition(0), GroupId(1)));
        assert_eq!(group_ids, GroupIds::from_iter(["group0", ""]));

        Ok(())
    }

    #[test]
    fn test_sample_from_str_unknown_name() -> Result<(), ParseSamplesError> {
        let header_positions = HeaderPositions::from_iter(["sample0"]);
        let mut group_ids = GroupIds::empty();

        let result = Sample::from_str("sample1", &header_positions, &mut group_ids);
        let expected = Err(ParseSamplesError::UnknownSample {
            sample_name: String::from("sample1"),
        });

        assert_eq!(result, expected);

        Ok(())
    }

    #[test]
    fn test_sample_list_from_str() -> Result<(), ParseSamplesError> {
        let s = "sample0\tgroup0
sample3\tgroup3
sample1\tgroup2
sample4\tgroup0";

        let header = vcf::Header::builder()
            .add_sample_name("sample0")
            .add_sample_name("sample1")
            .add_sample_name("sample2")
            .add_sample_name("sample3")
            .add_sample_name("sample4")
            .build();

        let sample_list = SampleList::from_str(s, &header)?;
        let expected = SampleList::from_iter([
            Some(GroupId(0)),
            Some(GroupId(2)),
            None,
            Some(GroupId(1)),
            Some(GroupId(0)),
        ]);

        assert_eq!(sample_list, expected);

        Ok(())
    }

    #[test]
    fn test_sample_list_from_str_empty() -> Result<(), ParseSamplesError> {
        let s = "";

        let header = vcf::Header::builder().add_sample_name("sample0").build();

        let result = SampleList::from_str(s, &header);

        assert_eq!(result, Err(ParseSamplesError::Empty));

        Ok(())
    }

    #[test]
    fn test_sample_list_from_str_default_group() -> Result<(), ParseSamplesError> {
        let s = "sample0\tgroup2
sample3
sample5
sample4\tgroup1";

        let header = vcf::Header::builder()
            .add_sample_name("sample0")
            .add_sample_name("sample1")
            .add_sample_name("sample2")
            .add_sample_name("sample3")
            .add_sample_name("sample4")
            .add_sample_name("sample5")
            .build();

        let sample_list = SampleList::from_str(s, &header)?;
        let expected = SampleList::from_iter([
            Some(GroupId(0)),
            None,
            None,
            Some(GroupId(1)),
            Some(GroupId(2)),
            Some(GroupId(1)),
        ]);

        assert_eq!(sample_list, expected);

        Ok(())
    }

    #[test]
    fn test_sample_list_shape() {
        let sample_list = SampleList::from_iter([
            Some(GroupId(0)),
            Some(GroupId(2)),
            None,
            Some(GroupId(1)),
            Some(GroupId(0)),
        ]);

        let expected = Shape(vec![5, 3, 3]);

        assert_eq!(sample_list.shape(), expected);
    }
}
