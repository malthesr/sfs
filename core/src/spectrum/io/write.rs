use std::{fs, io, path::Path};

use crate::{spectrum::State, Spectrum};

use super::{text, Format};

/// A builder to write a spectrum.
#[derive(Debug)]
pub struct Builder {
    format: Format,
    precision: usize,
}

impl Builder {
    /// Set format to write.
    ///
    /// If unset, the plain text format will be used.
    pub fn set_format(mut self, format: Format) -> Self {
        self.format = format;
        self
    }

    /// Set precision.
    ///
    /// This is only used for the plain text format.
    /// If unset, a precision of six digits will be used.
    pub fn set_precision(mut self, precision: usize) -> Self {
        self.precision = precision;
        self
    }

    /// Write spectrum to writer.
    pub fn write<W, S: State>(self, writer: &mut W, spectrum: &Spectrum<S>) -> io::Result<()>
    where
        W: io::Write,
    {
        match self.format {
            Format::Text => text::write_spectrum(writer, spectrum, self.precision),
            Format::Npy => spectrum.array.write_npy(writer),
        }
    }

    /// Write spectrum to stdout.
    pub fn write_to_stdout<S: State>(self, spectrum: &Spectrum<S>) -> io::Result<()> {
        self.write(&mut io::stdout().lock(), spectrum)
    }

    /// Write spectrum to path.
    ///
    /// If path already exists, it will be overwritten.
    pub fn write_to_path<P, S: State>(self, path: P, spectrum: &Spectrum<S>) -> io::Result<()>
    where
        P: AsRef<Path>,
    {
        self.write(&mut fs::File::create(path)?, spectrum)
    }

    /// Write spectrum to path or stdout.
    ///
    /// If the provided path is `None`, read from stdin.
    /// If path already exists, it will be overwritten.
    pub fn write_to_path_or_stdout<P, S: State>(
        self,
        path: Option<P>,
        spectrum: &Spectrum<S>,
    ) -> io::Result<()>
    where
        P: AsRef<Path>,
    {
        match path {
            Some(path) => self.write_to_path(path, spectrum),
            None => self.write_to_stdout(spectrum),
        }
    }
}

impl Default for Builder {
    fn default() -> Self {
        Builder {
            format: Format::Text,
            precision: 6,
        }
    }
}
