use std::{
    collections::HashSet,
    fmt, io,
    num::NonZeroUsize,
    path::{Path, PathBuf},
};

use crate::{
    array::Shape,
    spectrum::project::{PartialProjection, ProjectionError},
};

use super::{bcf::Reader as BcfReader, sample_map::Population, GenotypeReader, Reader, SampleMap};

#[derive(Debug)]
pub struct Builder {
    format: Option<Format>,
    sample_map: Option<SampleMap>,
    project_to: Option<Shape>,
    threads: NonZeroUsize,
}

impl Builder {
    fn build(self, reader: Box<dyn GenotypeReader>) -> Result<Reader, BuilderError> {
        let sample_map = if let Some(sample_map) = self.sample_map {
            sample_map
        } else {
            reader
                .samples()
                .iter()
                .map(|sample| (sample.0.to_string(), Population::Unnamed))
                .collect()
        };

        // All samples in sample map should be in reader samples
        let reader_samples = HashSet::<_>::from_iter(reader.samples().iter());
        if let Some(unknown_sample) = sample_map
            .samples()
            .find(|sample| !reader_samples.contains(sample))
        {
            return Err(BuilderError::UnknownSample {
                sample: unknown_sample.0.clone(),
            });
        }

        let projection = self
            .project_to
            .map(|project_to| {
                let project_from = sample_map.shape();

                if project_from.dimensions() != project_to.dimensions() {
                    Err(ProjectionError::UnequalDimensions {
                        from: project_from.dimensions(),
                        to: project_to.dimensions(),
                    })
                } else if let Some((dimension, (&from, &to))) = project_from
                    .iter()
                    .zip(project_to.iter())
                    .enumerate()
                    .find(|(_, (from, to))| from < to)
                {
                    Err(ProjectionError::InvalidProjection {
                        dimension,
                        from,
                        to,
                    })
                } else {
                    PartialProjection::from_shape(project_to)
                }
            })
            .transpose()?;

        Ok(Reader::new_unchecked(reader, sample_map, projection))
    }

    pub fn build_from_path<P>(self, path: P) -> Result<Reader, BuilderError>
    where
        P: AsRef<Path>,
    {
        match self.format {
            None | Some(Format::Bcf) => {
                let reader = BcfReader::from_path(path, self.threads).map(Box::new)?;

                self.build(reader)
            }
        }
    }

    pub fn build_from_path_or_stdin<P>(self, path: Option<P>) -> Result<Reader, BuilderError>
    where
        P: AsRef<Path>,
    {
        match path {
            Some(path) => self.build_from_path(path),
            None => self.build_from_stdin(),
        }
    }

    pub fn build_from_stdin(self) -> Result<Reader, BuilderError> {
        match self.format {
            None | Some(Format::Bcf) => {
                let reader = BcfReader::from_stdin(self.threads).map(Box::new)?;

                self.build(reader)
            }
        }
    }

    pub fn set_format(mut self, format: Format) -> Self {
        self.format = Some(format);
        self
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

    pub fn set_sample_map(mut self, sample_map: SampleMap) -> Result<Self, BuilderError> {
        if sample_map.is_empty() {
            Err(BuilderError::EmptySamplesMap)
        } else {
            self.sample_map = Some(sample_map);
            Ok(self)
        }
    }

    pub fn set_samples<I>(self, iter: I) -> Result<Self, BuilderError>
    where
        I: IntoIterator,
        SampleMap: FromIterator<I::Item>,
    {
        self.set_sample_map(iter.into_iter().collect())
    }

    pub fn set_samples_file<P>(self, samples_file: P) -> Result<Self, BuilderError>
    where
        P: AsRef<Path>,
    {
        let sample_map = SampleMap::from_path(samples_file)?;
        self.set_sample_map(sample_map)
    }

    pub fn set_threads(mut self, threads: NonZeroUsize) -> Self {
        self.threads = threads;
        self
    }
}

impl Default for Builder {
    fn default() -> Self {
        Self {
            format: None,
            sample_map: None,
            threads: NonZeroUsize::new(1).unwrap(),
            project_to: None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Format {
    Bcf,
}

#[derive(Debug)]
pub enum BuilderError {
    EmptySamplesMap,
    Io(io::Error),
    PathDoesNotExist { path: PathBuf },
    Projection(ProjectionError),
    UnknownSample { sample: String },
}

impl From<io::Error> for BuilderError {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<ProjectionError> for BuilderError {
    fn from(e: ProjectionError) -> Self {
        Self::Projection(e)
    }
}

impl fmt::Display for BuilderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BuilderError::EmptySamplesMap => f.write_str("empty samples mapping"),
            BuilderError::Io(e) => write!(f, "{e}"),
            BuilderError::PathDoesNotExist { path } => {
                write!(f, "path '{}' not found", path.display())
            }
            BuilderError::UnknownSample { sample } => write!(f, "unknown sample {sample}"),
            BuilderError::Projection(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for BuilderError {}
