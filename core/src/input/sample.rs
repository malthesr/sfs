//! Input samples.

use std::{collections::HashMap, fs::File, io, path::Path};

use indexmap::IndexMap;

use crate::array::Shape;

pub mod population;
pub use population::Population;

/// A sample.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Sample(String);

/// A numeric id for a sample.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Id(pub usize);

impl<S> From<S> for Sample
where
    S: ToString,
{
    fn from(sample: S) -> Self {
        Self(sample.to_string())
    }
}

impl AsRef<str> for Sample {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// A mapping from samples to populations.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Map(IndexMap<Sample, population::Id>);

impl Map {
    /// Creates a new mapping by mapping all samples to the same, unnamed population.
    pub fn from_all<I>(samples: I) -> Self
    where
        I: IntoIterator<Item = Sample>,
    {
        Self::from_iter(
            samples
                .into_iter()
                .map(|sample| (sample.as_ref().to_string(), Population::Unnamed)),
        )
    }

    /// Creates a new mapping by reading a samples file at the provided path.
    pub fn from_path<P>(path: P) -> io::Result<Self>
    where
        P: AsRef<Path>,
    {
        File::open(path).and_then(Self::from_reader)
    }

    /// Creates a new mapping by reading a samples file from the provided reader.
    pub fn from_reader<R>(mut reader: R) -> io::Result<Self>
    where
        R: io::Read,
    {
        let mut s = String::new();
        let _ = reader.read_to_string(&mut s)?;

        Ok(Self::from_str(&s))
    }

    fn from_str(s: &str) -> Self {
        s.lines()
            .map(|line| match line.split_once('\t') {
                Some((sample, population)) => (sample, Some(population)),
                None => (line, None),
            })
            .collect()
    }

    /// Returns the population id of a sample if defined, otherwise `None`.
    pub fn get_population_id(&self, sample: &Sample) -> Option<population::Id> {
        self.0.get(sample).copied()
    }

    /// Returns the sample with the provided id if defined, otherwise `None`.
    pub fn get_sample(&self, id: Id) -> Option<&Sample> {
        self.0.get_index(id.0).map(|opt| opt.0)
    }

    /// Returns the id of the provided sample if defined, otherwise `None`.
    pub fn get_sample_id(&self, sample: &Sample) -> Option<Id> {
        self.0.get_index_of(sample).map(Id)
    }

    /// Returns true if no samples are defined, false otherwise.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns the number of populations in the mapping.
    pub fn number_of_populations(&self) -> usize {
        self.population_sizes().len()
    }

    /// Returns the number of samples defined for each population id.
    pub fn population_sizes(&self) -> HashMap<population::Id, usize> {
        let mut sizes = HashMap::new();
        for &population_id in self.0.values() {
            *sizes.entry(population_id).or_insert(0) += 1;
        }
        sizes
    }

    /// Returns an iterator over the samples in the mapping.
    pub fn samples(&self) -> impl Iterator<Item = &Sample> {
        self.0.keys()
    }

    pub(crate) fn shape(&self) -> Shape {
        let population_sizes = self.population_sizes();

        Shape(
            (0..population_sizes.len())
                .map(|id| 1 + 2 * population_sizes.get(&population::Id(id)).unwrap())
                .collect(),
        )
    }
}

impl<S, P> FromIterator<(S, P)> for Map
where
    S: Into<Sample>,
    P: Into<Population>,
{
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = (S, P)>,
    {
        let mut population_map = population::Map::default();

        Self(IndexMap::from_iter(iter.into_iter().map(
            |(sample_name, population_name)| {
                (
                    sample_name.into(),
                    population_map.get_or_insert(population_name.into()),
                )
            },
        )))
    }
}
