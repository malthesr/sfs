use std::{
    collections::HashSet,
    fmt, io,
    path::{Path, PathBuf},
};

use crate::{
    array::Shape,
    input::{genotype, sample},
    spectrum::project::{PartialProjection, ProjectionError},
};

#[derive(Debug, Default)]
pub struct Builder {
    sample_map: Option<sample::Map>,
    project_to: Option<Shape>,
}

impl Builder {
    pub fn build(self, reader: genotype::reader::DynReader) -> Result<super::Reader, Error> {
        let sample_map = if let Some(sample_map) = self.sample_map {
            sample_map
        } else {
            reader
                .samples()
                .iter()
                .map(|sample| (sample.as_ref().to_string(), sample::Population::Unnamed))
                .collect()
        };

        // All samples in sample map should be in reader samples
        let reader_samples = HashSet::<_>::from_iter(reader.samples());
        if let Some(unknown_sample) = sample_map
            .samples()
            .find(|sample| !reader_samples.contains(sample))
        {
            return Err(Error::UnknownSample {
                sample: unknown_sample.as_ref().to_string(),
            });
        }

        let projection = if let Some(project_to) = self.project_to {
            let project_from = sample_map.shape();

            if project_from.dimensions() != project_to.dimensions() {
                return Err(ProjectionError::UnequalDimensions {
                    from: project_from.dimensions(),
                    to: project_to.dimensions(),
                }
                .into());
            } else if let Some((dimension, (&from, &to))) = project_from
                .iter()
                .zip(project_to.iter())
                .enumerate()
                .find(|(_, (from, to))| from < to)
            {
                return Err(ProjectionError::InvalidProjection {
                    dimension,
                    from,
                    to,
                }
                .into());
            } else {
                Some(PartialProjection::from_shape(project_to)?)
            }
        } else {
            None
        };

        Ok(super::Reader::new_unchecked(reader, sample_map, projection))
    }

    pub fn set_project_individuals(self, individuals: Vec<usize>) -> Self {
        self.set_project_shape(Shape(individuals.into_iter().map(|i| 2 * i + 1).collect()))
    }

    pub fn set_project_shape<S>(mut self, shape: S) -> Self
    where
        S: Into<Shape>,
    {
        self.project_to = Some(shape.into());
        self
    }

    pub fn set_sample_map(mut self, sample_map: sample::Map) -> Result<Self, Error> {
        if sample_map.is_empty() {
            Err(Error::EmptySamplesMap)
        } else {
            self.sample_map = Some(sample_map);
            Ok(self)
        }
    }

    pub fn set_samples<I>(self, iter: I) -> Result<Self, Error>
    where
        I: IntoIterator,
        sample::Map: FromIterator<I::Item>,
    {
        self.set_sample_map(iter.into_iter().collect())
    }

    pub fn set_samples_file<P>(self, samples_file: P) -> Result<Self, Error>
    where
        P: AsRef<Path>,
    {
        let sample_map = sample::Map::from_path(samples_file)?;
        self.set_sample_map(sample_map)
    }
}

#[derive(Debug)]
pub enum Error {
    EmptySamplesMap,
    Io(io::Error),
    PathDoesNotExist { path: PathBuf },
    Projection(ProjectionError),
    UnknownSample { sample: String },
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<ProjectionError> for Error {
    fn from(e: ProjectionError) -> Self {
        Self::Projection(e)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::EmptySamplesMap => f.write_str("empty samples mapping"),
            Error::Io(e) => write!(f, "{e}"),
            Error::PathDoesNotExist { path } => {
                write!(f, "path '{}' not found", path.display())
            }
            Error::UnknownSample { sample } => write!(f, "unknown sample {sample}"),
            Error::Projection(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for Error {}
