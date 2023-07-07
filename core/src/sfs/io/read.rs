//! Utilities for reading SFS.

use std::{fs, io, path::Path};

use crate::{Array, Sfs};

use super::{text, Format};

/// A builder to read an SFS.
#[derive(Debug, Default)]
pub struct Builder {
    format: Option<Format>,
}

impl Builder {
    /// Read SFS from reader.
    pub fn read<R>(self, reader: &mut R) -> io::Result<Sfs>
    where
        R: io::Read,
    {
        let mut raw = Vec::new();
        reader.read_to_end(&mut raw)?;

        let format = self.format.or_else(|| Format::detect(&raw));

        let reader = &mut &raw[..];
        match format {
            Some(Format::Text) => text::read_sfs(reader),
            Some(Format::Npy) => Array::read_npy(reader).map(Sfs::from),
            None => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "invalid SFS format",
            )),
        }
    }

    /// Read SFS from path.
    pub fn read_from_path<P>(self, path: P) -> io::Result<Sfs>
    where
        P: AsRef<Path>,
    {
        self.read(&mut fs::File::open(path)?)
    }

    /// Read SFS from path or stdin.
    ///
    /// If the provided path is `None`, read from stdin.
    pub fn read_from_path_or_stdin<P>(self, path: Option<P>) -> io::Result<Sfs>
    where
        P: AsRef<Path>,
    {
        match path {
            Some(path) => self.read_from_path(path),
            None => self.read_from_stdin(),
        }
    }

    /// Read SFS from stdin.
    pub fn read_from_stdin(self) -> io::Result<Sfs> {
        self.read(&mut io::stdin().lock())
    }

    /// Set SFS format to read.
    ///
    /// If unset, the format will automatically be detected when reading.
    pub fn set_format(mut self, format: Format) -> Self {
        self.format = Some(format);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::array::npy;

    #[test]
    fn test_detect_npy() {
        assert_eq!(Format::detect_npy(&npy::MAGIC), Some(Format::Npy));

        let mut bytes = npy::MAGIC.to_vec();
        bytes.extend(b"foobar");
        assert_eq!(Format::detect(&bytes), Some(Format::Npy));
    }

    #[test]
    fn test_detect_plain_text() {
        assert_eq!(Format::detect_plain_text(&text::START), Some(Format::Text));

        let mut bytes = text::START.to_vec();
        bytes.extend(b"=<17/19>\n1 2 3");
        assert_eq!(Format::detect(&bytes), Some(Format::Text));
    }
}
