use std::{
    collections::HashMap,
    fmt,
    fs::File,
    io::{self, Read},
    path::Path,
    slice,
    str::FromStr,
};

use indexmap::IndexSet;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OrderedSampleList {
    inner: Vec<Option<usize>>,
}

impl OrderedSampleList {
    pub fn iter_groups(&self) -> slice::Iter<Option<usize>> {
        self.inner.iter()
    }
}

impl OrderedSampleList {
    pub fn from_map_and_ordered_samples(map: &SampleMap, ordered_samples: &[String]) -> Self {
        Self {
            inner: ordered_samples
                .iter()
                .map(|sample| map.inner.get(sample).copied())
                .collect(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SampleMap {
    inner: HashMap<String, usize>,
}

impl SampleMap {
    pub fn from_path<P>(path: P) -> io::Result<Result<Self, ParseSamplesError>>
    where
        P: AsRef<Path>,
    {
        let mut file = File::open(path)?;
        let mut s = String::new();
        let _ = file.read_to_string(&mut s)?;

        Ok(Self::from_str(&s))
    }

    pub fn sample_names(&self) -> impl Iterator<Item = &String> + '_ {
        self.inner.keys()
    }

    fn from_samples(samples: Vec<Sample>) -> Result<Self, ParseSamplesError> {
        if samples.is_empty() {
            Err(ParseSamplesError::Empty)
        } else {
            Ok(Self {
                inner: samples
                    .into_iter()
                    .map(|sample| (sample.name, sample.group_id))
                    .collect(),
            })
        }
    }

    pub fn from_names_and_group_names(
        names: Vec<(String, Option<String>)>,
    ) -> Result<Self, ParseSamplesError> {
        let mut groups = Groups::new();

        let samples = names
            .into_iter()
            .map(|(name, group)| Sample::from_name_and_group(name, group.as_deref(), &mut groups))
            .collect::<Result<Vec<_>, _>>()?;

        Self::from_samples(samples)
    }

    pub fn from_names_in_single_group(names: Vec<String>) -> Self {
        Self {
            inner: names.into_iter().map(|name| (name, 0)).collect(),
        }
    }
}

impl FromStr for SampleMap {
    type Err = ParseSamplesError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut groups = Groups::new();

        let samples = s
            .lines()
            .map(|line| Sample::from_str(line, &mut groups))
            .collect::<Result<Vec<_>, _>>()?;

        Self::from_samples(samples)
    }
}

impl<T> FromIterator<T> for SampleMap
where
    HashMap<String, usize>: FromIterator<T>,
{
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = T>,
    {
        Self {
            inner: HashMap::from_iter(iter),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Sample {
    name: String,
    group_id: usize,
}

impl Sample {
    fn new(name: String, group_id: usize) -> Self {
        Self { name, group_id }
    }

    fn from_name_and_group(
        name: String,
        group: Option<&str>,
        groups: &mut Groups,
    ) -> Result<Self, ParseSamplesError> {
        let group = group.unwrap_or_default();

        let group_id = groups
            .get_id_of(group)
            .unwrap_or_else(|| groups.create_group(group));

        Ok(Self::new(name, group_id))
    }

    fn from_str(s: &str, groups: &mut Groups) -> Result<Self, ParseSamplesError> {
        let (name, group) = match s.split_once('\t') {
            Some((name, group)) => (name, Some(group)),
            None => (s, None),
        };

        Self::from_name_and_group(name.to_string(), group, groups)
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct Groups(IndexSet<String>);

impl Groups {
    pub fn create_group<S>(&mut self, group_name: S) -> usize
    where
        S: ToString,
    {
        self.0.insert_full(group_name.to_string()).0
    }

    pub fn new() -> Self {
        Self(IndexSet::new())
    }

    fn get_id_of(&self, group_name: &str) -> Option<usize> {
        self.0.get_index_of(group_name)
    }
}

impl<S> FromIterator<S> for Groups
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ParseSamplesError {
    Empty,
}

impl fmt::Display for ParseSamplesError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseSamplesError::Empty => f.write_str("empty samples list"),
        }
    }
}

impl std::error::Error for ParseSamplesError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sample_from_str() -> Result<(), ParseSamplesError> {
        let mut groups = Groups::new();

        let sample = Sample::from_str("sample0\tgroup1", &mut groups)?;

        assert_eq!(sample, Sample::new(String::from("sample0"), 0));
        assert_eq!(groups, Groups::from_iter(["group1"]));

        Ok(())
    }

    #[test]
    fn test_sample_from_str_default_group() -> Result<(), ParseSamplesError> {
        let mut groups = Groups::from_iter(["group0"]);

        let sample = Sample::from_str("sample0", &mut groups)?;

        assert_eq!(sample, Sample::new(String::from("sample0"), 1));
        assert_eq!(groups, Groups::from_iter(["group0", ""]));

        Ok(())
    }

    #[test]
    fn test_sample_list_from_str() {
        let s = "sample0\tgroup0
sample3\tgroup3
sample1\tgroup2
sample4\tgroup0";

        let expected = SampleMap::from_iter([
            (String::from("sample0"), 0),
            (String::from("sample3"), 1),
            (String::from("sample1"), 2),
            (String::from("sample4"), 0),
        ]);

        assert_eq!(SampleMap::from_str(s), Ok(expected));
    }

    #[test]
    fn test_sample_list_from_str_empty() {
        assert_eq!(SampleMap::from_str(""), Err(ParseSamplesError::Empty));
    }

    #[test]
    fn test_sample_list_from_str_default_group() {
        let s = "sample0\tgroup2
sample3
sample5
sample4\tgroup1";

        let expected = SampleMap::from_iter([
            (String::from("sample0"), 0),
            (String::from("sample3"), 1),
            (String::from("sample5"), 1),
            (String::from("sample4"), 2),
        ]);

        assert_eq!(SampleMap::from_str(s), Ok(expected));
    }
}
