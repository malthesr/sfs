use std::{collections::HashMap, fs::File, io, path::Path};

use indexmap::{IndexMap, IndexSet};

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Sample(pub String);

impl<S> From<S> for Sample
where
    S: ToString,
{
    fn from(sample: S) -> Self {
        Self(sample.to_string())
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum Population {
    Named(String),
    Unnamed,
}

impl<S> From<Option<S>> for Population
where
    S: ToString,
{
    fn from(population: Option<S>) -> Self {
        match population {
            Some(population) => Self::Named(population.to_string()),
            None => Self::Unnamed,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PopulationId(pub usize);

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SampleMap(IndexMap<Sample, PopulationId>);

impl SampleMap {
    pub fn from_path<P>(path: P) -> io::Result<Self>
    where
        P: AsRef<Path>,
    {
        File::open(path).and_then(Self::from_reader)
    }

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

    pub fn get(&self, sample: &Sample) -> Option<PopulationId> {
        self.0.get(sample).copied()
    }

    pub fn number_of_populations(&self) -> usize {
        self.population_sizes().len()
    }

    pub fn population_sizes(&self) -> HashMap<PopulationId, usize> {
        let mut sizes = HashMap::new();
        for &population_id in self.0.values() {
            *sizes.entry(population_id).or_insert(0) += 1;
        }
        sizes
    }

    pub fn samples(&self) -> impl Iterator<Item = &Sample> {
        self.0.keys()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl<S, P> FromIterator<(S, P)> for SampleMap
where
    S: Into<Sample>,
    P: Into<Population>,
{
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = (S, P)>,
    {
        let mut population_map = PopulationMap::default();

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

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct PopulationMap(IndexSet<Population>);

impl PopulationMap {
    pub fn get(&self, name: &Population) -> Option<PopulationId> {
        self.0.get_index_of(name).map(PopulationId)
    }

    pub fn get_or_insert(&mut self, name: Population) -> PopulationId {
        self.get(&name).unwrap_or_else(|| self.insert(name))
    }

    pub fn insert(&mut self, name: Population) -> PopulationId {
        PopulationId(self.0.insert_full(name).0)
    }
}
