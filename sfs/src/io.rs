use std::{
    fmt::{self, Write},
    io,
};

use super::{Sfs, Shape};

pub fn format_sfs(sfs: &Sfs, sep: &str, precision: usize) -> String {
    if let Some(first) = sfs.as_slice().first() {
        let mut init = String::new();
        write!(init, "{first:.precision$}").unwrap();

        sfs.as_slice().iter().skip(1).fold(init, |mut s, x| {
            s.push_str(sep);
            write!(s, "{x:.precision$}").unwrap();
            s
        })
    } else {
        String::new()
    }
}

pub fn write_sfs<W>(writer: &mut W, sfs: &Sfs, sep: &str, precision: usize) -> io::Result<()>
where
    W: io::Write,
{
    let header = Header::new(sfs.shape().clone());
    header.write(writer)?;

    writeln!(writer, "{}", format_sfs(sfs, sep, precision))
}

#[derive(Clone, Debug)]
struct Header {
    shape: Shape,
}

impl Header {
    pub fn new(shape: Shape) -> Self {
        Self { shape }
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
