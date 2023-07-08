//! Reading and writing for the text format.
//!
//! The plain text format is a simple format consisting of two lines.
//! The first line contains a header line `#SHAPE=<[shape]>`, where `[shape]`
//! is a `/`-separated representation of the shape of the spectrum. The next line
//! gives the spectrum in flat, row-major order separated by a single space.

use std::{
    fmt::{self, Write},
    io,
    str::FromStr,
};

use crate::{
    spectrum::{Shape, State},
    Scs, Spectrum,
};

/// The text format start string.
pub(crate) const START: [u8; 6] = *b"#SHAPE";

fn parse_scs(s: &str, shape: Shape) -> io::Result<Scs> {
    s.split_ascii_whitespace()
        .map(f64::from_str)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
        .and_then(|vec| {
            Scs::new(vec, shape).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
        })
}

/// Reads an SCS in text format from a reader.
///
/// The stream is assumed to be positioned at the start.
pub fn read_scs<R>(reader: &mut R) -> io::Result<Scs>
where
    R: io::BufRead,
{
    let header = Header::read(reader)?;

    let mut buf = String::new();
    let _bytes_read = reader.read_to_string(&mut buf)?;

    parse_scs(&buf, header.shape)
}

fn format_spectrum<S: State>(spectrum: &Spectrum<S>, sep: &str, precision: usize) -> String {
    if let Some(first) = spectrum.array.as_slice().first() {
        let mut init = String::new();
        write!(init, "{first:.precision$}").unwrap();

        spectrum
            .array
            .as_slice()
            .iter()
            .skip(1)
            .fold(init, |mut s, x| {
                s.push_str(sep);
                write!(s, "{x:.precision$}").unwrap();
                s
            })
    } else {
        String::new()
    }
}

/// Writes a spectrum in text format to a writer.
pub fn write_spectrum<W, S: State>(
    writer: &mut W,
    spectrum: &Spectrum<S>,
    precision: usize,
) -> io::Result<()>
where
    W: io::Write,
{
    let header = Header::new(spectrum.array.shape().clone());
    header.write(writer)?;

    writeln!(writer, "{}", format_spectrum(spectrum, " ", precision))
}

#[derive(Clone, Debug)]
struct Header {
    shape: Shape,
}

impl Header {
    pub fn new(shape: Shape) -> Self {
        Self { shape }
    }

    pub fn read<R>(reader: &mut R) -> io::Result<Self>
    where
        R: io::BufRead,
    {
        let mut buf = String::new();

        reader.read_line(&mut buf)?;

        Self::from_str(&buf).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    pub fn write<W>(&self, writer: &mut W) -> io::Result<()>
    where
        W: io::Write,
    {
        writeln!(writer, "{self}")
    }
}

impl fmt::Display for Header {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let shape_fmt = self
            .shape
            .iter()
            .map(|x| x.to_string())
            .collect::<Vec<_>>()
            .join("/");

        write!(f, "#SHAPE=<{shape_fmt}>")
    }
}

impl FromStr for Header {
    type Err = ParseHeaderError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.trim_start_matches(|c: char| !c.is_numeric())
            .trim_end_matches(|c: char| !c.is_numeric())
            .split('/')
            .map(usize::from_str)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| ParseHeaderError(String::from(s)))
            .map(Shape)
            .map(Header::new)
    }
}

/// An error associated with parsing the plain text format header.
#[derive(Debug)]
pub struct ParseHeaderError(String);

impl fmt::Display for ParseHeaderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "failed to parse '{}' as plain text format header",
            self.0
        )
    }
}

impl std::error::Error for ParseHeaderError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_header() {
        assert_eq!(Header::from_str("#SHAPE=<3>").unwrap().shape.as_ref(), [3]);
        assert_eq!(
            Header::from_str("#SHAPE=<11/13>").unwrap().shape.as_ref(),
            &[11, 13]
        );
    }

    #[test]
    fn test_display_header() {
        assert_eq!(Header::new(Shape(vec![25])).to_string(), "#SHAPE=<25>");
        assert_eq!(Header::new(Shape(vec![7, 9])).to_string(), "#SHAPE=<7/9>");
    }

    #[test]
    fn test_read_1d() -> io::Result<()> {
        let src = b"#SHAPE=<3>\n0.0 1.0 2.0\n";

        assert_eq!(read_scs(&mut &src[..])?, Scs::new([0., 1., 2.], 3).unwrap());

        Ok(())
    }

    #[test]
    fn test_read_2d() -> io::Result<()> {
        let src = b"#SHAPE=<2/3>\n0.0 1.0 2.0 3.0 4.0 5.0\n";

        assert_eq!(
            read_scs(&mut &src[..])?,
            Scs::new([0., 1., 2., 3., 4., 5.], [2, 3]).unwrap()
        );

        Ok(())
    }

    #[test]
    fn test_write_1d() -> io::Result<()> {
        let mut dest = Vec::new();
        write_spectrum(&mut dest, &Scs::new([0., 1., 2.], 3).unwrap(), 2)?;

        assert_eq!(dest, b"#SHAPE=<3>\n0.00 1.00 2.00\n");

        Ok(())
    }

    #[test]
    fn test_write_2d() -> io::Result<()> {
        let mut dest = Vec::new();
        write_spectrum(
            &mut dest,
            &Scs::new([0., 1., 2., 3., 4., 5.], [2, 3]).unwrap(),
            6,
        )?;

        assert_eq!(
            dest,
            b"#SHAPE=<2/3>\n0.000000 1.000000 2.000000 3.000000 4.000000 5.000000\n",
        );

        Ok(())
    }
}
