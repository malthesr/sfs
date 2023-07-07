use std::{fs, io, path::Path};

use crate::Sfs;

use super::{text, Format};

/// A builder to write an SFS.
#[derive(Debug)]
pub struct Builder {
    format: Format,
    precision: usize,
}

impl Builder {
    /// Set SFS format to write.
    ///
    /// If unset, the plain text format will be used.
    pub fn set_format(mut self, format: Format) -> Self {
        self.format = format;
        self
    }

    /// Set SFS precision.
    ///
    /// This is only used for the plain text format.
    /// If unset, a precision of six digits will be used.
    pub fn set_precision(mut self, precision: usize) -> Self {
        self.precision = precision;
        self
    }

    /// Write SFS to writer.
    pub fn write<W, const N: bool>(self, writer: &mut W, sfs: &Sfs<N>) -> io::Result<()>
    where
        W: io::Write,
    {
        match self.format {
            Format::Text => text::write_sfs(writer, sfs, self.precision),
            Format::Npy => sfs.array.write_npy(writer),
        }
    }

    /// Write SFS to stdout.
    pub fn write_to_stdout<const N: bool>(self, sfs: &Sfs<N>) -> io::Result<()> {
        self.write(&mut io::stdout().lock(), sfs)
    }

    /// Write SFS to path.
    ///
    /// If path already exists, it will be overwritten.
    pub fn write_to_path<P, const N: bool>(self, path: P, sfs: &Sfs<N>) -> io::Result<()>
    where
        P: AsRef<Path>,
    {
        self.write(&mut fs::File::create(path)?, sfs)
    }

    /// Write SFS to path or stdout.
    ///
    /// If the provided path is `None`, read from stdin.
    /// If path already exists, it will be overwritten.
    pub fn write_to_path_or_stdout<P, const N: bool>(
        self,
        path: Option<P>,
        sfs: &Sfs<N>,
    ) -> io::Result<()>
    where
        P: AsRef<Path>,
    {
        match path {
            Some(path) => self.write_to_path(path, sfs),
            None => self.write_to_stdout(sfs),
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
