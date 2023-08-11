//! Utilities for reading SCS.

use std::io::{self, Read};

use crate::{Array, Input, Scs};

use super::{text, Format};

/// A builder to read an SCS.
#[derive(Debug, Default)]
pub struct Builder {
    input: Option<Input>,
    format: Option<Format>,
}

impl Builder {
    /// Read SCS from reader.
    pub fn read(self) -> io::Result<Scs> {
        let mut raw = Vec::new();

        _ = match self.input.unwrap_or(Input::Stdin).open()? {
            crate::input::Reader::File(mut reader) => reader.read_to_end(&mut raw)?,
            crate::input::Reader::Stdin(mut reader) => reader.read_to_end(&mut raw)?,
        };

        let format = self.format.or_else(|| Format::detect(&raw));

        let reader = &mut &raw[..];
        match format {
            Some(Format::Text) => text::read_scs(reader),
            Some(Format::Npy) => Array::read_npy(reader).map(Scs::from),
            None => Err(io::Error::new(io::ErrorKind::InvalidData, "invalid format")),
        }
    }

    /// Set input source.
    ///
    /// If unset, the input source will default to stdin.
    pub fn set_input(mut self, input: Input) -> Self {
        self.input = Some(input);
        self
    }

    /// Set format to read.
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
