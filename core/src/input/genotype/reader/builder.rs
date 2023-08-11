use std::{
    io::{self, Read as _},
    num::NonZeroUsize,
};

use flate2::bufread::MultiGzDecoder;

use noodles_bgzf as bgzf;

use crate::{input, Input};

#[derive(Debug)]
pub struct Builder {
    input: Option<Input>,
    format: Option<Format>,
    compression_method: Option<Option<CompressionMethod>>,
    threads: NonZeroUsize,
}

impl Default for Builder {
    fn default() -> Self {
        Self {
            input: None,
            format: None,
            compression_method: None,
            threads: NonZeroUsize::try_from(4).unwrap(),
        }
    }
}

impl Builder {
    pub fn build(self) -> io::Result<super::DynReader> {
        match self.input.as_ref().unwrap_or(&Input::Stdin).open()? {
            input::Reader::File(reader) => self.build_from_reader(reader),
            input::Reader::Stdin(reader) => self.build_from_reader(reader),
        }
    }

    fn build_from_reader<R>(self, mut reader: R) -> io::Result<super::DynReader>
    where
        R: 'static + io::BufRead,
    {
        let compression_method = match self.compression_method {
            Some(compression_method) => compression_method,
            None => CompressionMethod::detect(&mut reader)?,
        };

        let format = match self.format {
            Some(format) => format,
            None => Format::detect(&mut reader, compression_method)?,
        };

        let reader: super::DynReader = match compression_method {
            Some(CompressionMethod::Bgzf) => {
                let bgzf_reader = bgzf::reader::Builder::default()
                    .set_worker_count(self.threads)
                    .build_from_reader(reader);

                match format {
                    Format::Bcf => super::bcf::Reader::new(bgzf_reader).map(Box::new)?,
                    Format::Vcf => super::vcf::Reader::new(bgzf_reader).map(Box::new)?,
                }
            }
            None => match format {
                Format::Bcf => super::bcf::Reader::new(reader).map(Box::new)?,
                Format::Vcf => super::vcf::Reader::new(reader).map(Box::new)?,
            },
        };

        Ok(reader)
    }

    pub fn set_compression_method(mut self, compression_method: Option<CompressionMethod>) -> Self {
        self.compression_method = Some(compression_method);
        self
    }

    pub fn set_format(mut self, format: Format) -> Self {
        self.format = Some(format);
        self
    }

    pub fn set_input(mut self, input: Input) -> Self {
        self.input = Some(input);
        self
    }

    pub fn set_threads(mut self, threads: NonZeroUsize) -> Self {
        self.threads = threads;
        self
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
