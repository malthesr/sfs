//! Site reader builder.

use std::{collections::HashSet, fmt, io, path::PathBuf};

use sample::Sample;

use crate::{
    array::Shape,
    input::{genotype, sample},
    spectrum::project::{PartialProjection, ProjectionError},
};

/// A site reader builder.
#[derive(Debug, Default)]
pub struct Builder {
    samples: Option<Option<Samples>>,
    project: Option<Option<Project>>,
}

impl Builder {
    /// Returns a new reader based on the provided genotype reader.
    ///
    /// # Errors
    ///
    /// For a variety of reasons, see [`Error`] for details.
    pub fn build(self, reader: genotype::reader::DynReader) -> Result<super::Reader, Error> {
        let sample_map = match self.samples.unwrap_or(None) {
            Some(Samples::List(list)) => sample::Map::from_iter(list),
            Some(Samples::Path(path)) => sample::Map::from_path(path)?,
            None => sample::Map::from_all(reader.samples().iter().cloned()),
        };

        if sample_map.is_empty() {
            return Err(Error::EmptySamplesMap);
        }

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

        let projection = if let Some(project_to) = self.project.unwrap_or(None).map(Project::shape)
        {
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

    /// Sets the projection used for reading.
    ///
    /// By default, no projection will be used.
    pub fn set_project(mut self, project: Option<Project>) -> Self {
        self.project = Some(project);
        self
    }

    /// Sets the sample mapping used for reading.
    ///
    /// By default, all samples will be mapped to the same, unnamed population.
    pub fn set_samples(mut self, samples: Option<Samples>) -> Self {
        self.samples = Some(samples);
        self
    }
}

/// A source for a sample mapping.
#[derive(Debug)]
pub enum Samples {
    /// A path to a samples file.
    Path(PathBuf),
    /// A list of samples and associated populations.
    List(Vec<(Sample, sample::Population)>),
}

/// A projection specification.
#[derive(Debug)]
pub enum Project {
    /// Project to specified number of individuals.
    Individuals(Vec<usize>),
    /// Project to specified shape.
    Shape(Shape),
}

impl Project {
    fn shape(self) -> Shape {
        match self {
            Project::Individuals(individuals) => {
                Shape(individuals.into_iter().map(|i| 2 * i + 1).collect())
            }
            Project::Shape(shape) => shape,
        }
    }
}

/// An error associated with building a site reader.
#[derive(Debug)]
pub enum Error {
    /// Provided sample mappping is empty.
    EmptySamplesMap,
    /// I/O error.
    Io(io::Error),
    /// A provided path does not exist.
    PathDoesNotExist {
        /// The provided path.
        path: PathBuf,
    },
    /// A projection error.
    Projection(ProjectionError),
    /// Provided sample mapping defines a sample not defined by the genotype reader.
    UnknownSample {
        /// The unknown sample.
        sample: String,
    },
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
