use std::{
    collections::HashSet,
    fmt,
    fs::File,
    io::{self, Read},
    num::NonZeroUsize,
    path::{Path, PathBuf},
};

use flate2::bufread::MultiGzDecoder;

use noodles_bgzf as bgzf;

use crate::{
    array::Shape,
    spectrum::project::{PartialProjection, ProjectionError},
};

use super::{
    bcf::Reader as BcfReader,
    sample_map::{Population, Sample},
    vcf::Reader as VcfReader,
    GenotypeReader, Reader, SampleMap,
};

#[derive(Debug)]
pub struct Builder {
    path: Option<PathBuf>,
    format: Option<Format>,
    compression_method: Option<Option<CompressionMethod>>,
    sample_map: Option<SampleMap>,
    project_to: Option<Shape>,
    threads: NonZeroUsize,
}

impl Builder {
    pub fn build(self) -> Result<Reader, BuilderError> {
        let Builder {
            path,
            format,
            compression_method,
            sample_map,
            project_to,
            threads,
        } = self;

        let reader = Self::build_reader(path, format, compression_method, threads)?;
        let sample_map = Self::build_sample_map(sample_map, reader.samples())?;
        let projection = Self::build_projection(project_to, &sample_map)?;

        Ok(Reader::new_unchecked(reader, sample_map, projection))
    }

    fn build_projection(
        project_to: Option<Shape>,
        sample_map: &SampleMap,
    ) -> Result<Option<PartialProjection>, BuilderError> {
        project_to
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
            .transpose()
            .map_err(BuilderError::from)
    }

    fn build_reader(
        path: Option<PathBuf>,
        format: Option<Format>,
        compression_method: Option<Option<CompressionMethod>>,
        threads: NonZeroUsize,
    ) -> io::Result<Box<dyn GenotypeReader>> {
        fn format_and_compression_method<R>(
            reader: &mut R,
            format: Option<Format>,
            compression_method: Option<Option<CompressionMethod>>,
        ) -> io::Result<(Format, Option<CompressionMethod>)>
        where
            R: io::BufRead,
        {
            let compression_method = match compression_method {
                Some(compression_method) => compression_method,
                None => CompressionMethod::detect(reader)?,
            };

            let format = match format {
                Some(format) => format,
                None => Format::detect(reader, compression_method)?,
            };

            Ok((format, compression_method))
        }

        fn genotype_reader<R>(
            reader: R,
            format: Format,
            compression_method: Option<CompressionMethod>,
            threads: NonZeroUsize,
        ) -> io::Result<Box<dyn GenotypeReader>>
        where
            R: 'static + io::BufRead,
        {
            let reader: Box<dyn GenotypeReader> = match compression_method {
                Some(CompressionMethod::Bgzf) => {
                    let bgzf_reader = bgzf::reader::Builder::default()
                        .set_worker_count(threads)
                        .build_from_reader(reader);

                    match format {
                        Format::Bcf => BcfReader::new(bgzf_reader).map(Box::new)?,
                        Format::Vcf => VcfReader::new(bgzf_reader).map(Box::new)?,
                    }
                }
                None => match format {
                    Format::Bcf => BcfReader::new(reader).map(Box::new)?,
                    Format::Vcf => VcfReader::new(reader).map(Box::new)?,
                },
            };

            Ok(reader)
        }

        if let Some(path) = path.as_ref() {
            let mut reader = File::open(path).map(io::BufReader::new)?;

            let (format, compression_method) =
                format_and_compression_method(&mut reader, format, compression_method)?;

            genotype_reader(reader, format, compression_method, threads)
        } else {
            let mut reader = io::stdin().lock();

            let (format, compression_method) =
                format_and_compression_method(&mut reader, format, compression_method)?;

            genotype_reader(reader, format, compression_method, threads)
        }
    }

    fn build_sample_map(
        sample_map: Option<SampleMap>,
        samples: &[Sample],
    ) -> Result<SampleMap, BuilderError> {
        let sample_map = if let Some(sample_map) = sample_map {
            sample_map
        } else {
            samples
                .iter()
                .map(|sample| (sample.as_ref().to_string(), Population::Unnamed))
                .collect()
        };

        // All samples in sample map should be in reader samples
        let reader_samples = HashSet::<_>::from_iter(samples);
        if let Some(unknown_sample) = sample_map
            .samples()
            .find(|sample| !reader_samples.contains(sample))
        {
            return Err(BuilderError::UnknownSample {
                sample: unknown_sample.as_ref().to_string(),
            });
        }

        Ok(sample_map)
    }

    pub fn set_format(mut self, format: Format) -> Self {
        self.format = Some(format);
        self
    }

    pub fn set_path(mut self, path: PathBuf) -> Self {
        self.path = Some(path);
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
            path: None,
            format: None,
            compression_method: None,
            sample_map: None,
            threads: NonZeroUsize::new(1).unwrap(),
            project_to: None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Format {
    Bcf,
    Vcf,
}

impl Format {
    fn detect<R>(
        reader: &mut R,
        compression_method: Option<CompressionMethod>,
    ) -> io::Result<Format>
    where
        R: io::BufRead,
    {
        const BCF_MAGIC_NUMBER: [u8; 3] = *b"BCF";

        let src = reader.fill_buf()?;

        if let Some(compression_method) = compression_method {
            if compression_method == CompressionMethod::Bgzf {
                let mut decoder = MultiGzDecoder::new(src);
                let mut buf = [0; BCF_MAGIC_NUMBER.len()];
                decoder.read_exact(&mut buf)?;

                if buf == BCF_MAGIC_NUMBER {
                    return Ok(Format::Bcf);
                }
            }
        } else if let Some(buf) = src.get(..BCF_MAGIC_NUMBER.len()) {
            if buf == BCF_MAGIC_NUMBER {
                return Ok(Format::Bcf);
            }
        }

        Ok(Format::Vcf)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CompressionMethod {
    Bgzf,
}

impl CompressionMethod {
    fn detect<R>(reader: &mut R) -> io::Result<Option<Self>>
    where
        R: io::BufRead,
    {
        const GZIP_MAGIC_NUMBER: [u8; 2] = [0x1f, 0x8b];

        let src = reader.fill_buf()?;

        if let Some(buf) = src.get(..GZIP_MAGIC_NUMBER.len()) {
            if buf == GZIP_MAGIC_NUMBER {
                return Ok(Some(CompressionMethod::Bgzf));
            }
        }

        Ok(None)
    }
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
