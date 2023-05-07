use std::{fmt, path::PathBuf};

use anyhow::Error;

use clap::{Parser, ValueEnum};

use sfs_core::sfs::Axis;

/// Format, marginalize, and convert SFS.
#[derive(Debug, Parser)]
pub struct View {
    /// Input SFS.
    ///
    /// The input SFS can be provided here or read from stdin in any of the supported formats.
    #[clap(value_parser, value_name = "PATH")]
    pub path: Option<PathBuf>,

    /// Marginalize out populations except those with the provided 0-based indices.
    ///
    /// The indices correspond to the ordering of the dimensions of the SFS in the same way as the
    /// shape.
    #[clap(short = 'M', long, use_value_delimiter = true, value_name = "INT,...")]
    pub marginalize_keep: Option<Vec<usize>>,

    /// Marginalize out populations with the provided 0-based indices.
    ///
    /// The indices correspond to the ordering of the dimensions of the SFS in the same way as the
    /// shape.
    #[clap(
        short = 'm',
        long,
        conflicts_with = "marginalize_keep",
        use_value_delimiter = true,
        value_name = "INT,..."
    )]
    pub marginalize_remove: Option<Vec<usize>>,

    /// Output SFS path.
    ///
    /// If no path is given, SFS will be output to stdout.
    #[clap(short = 'o', long, value_name = "PATH")]
    pub output: Option<PathBuf>,

    /// Output SFS format.
    #[clap(short = 'O', long, default_value_t = Format::Text, value_name = "FORMAT")]
    pub output_format: Format,

    /// Precision to use when printing SFS.
    ///
    /// This is only used for printing SFS to plain text format, and will be ignored otherwise.
    #[clap(short = 'p', long, default_value_t = 6, value_name = "INT")]
    pub precision: usize,
}

#[derive(ValueEnum, Clone, Copy, Debug, Eq, PartialEq)]
pub enum Format {
    // Binary numpy npy format.
    Npy,
    // Plain text format.
    Text,
}

impl Format {
    pub fn name(&self) -> &'static str {
        match self {
            Format::Npy => "npy",
            Format::Text => "text",
        }
    }
}

impl fmt::Display for Format {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

impl From<Format> for sfs_core::sfs::io::Format {
    fn from(value: Format) -> Self {
        match value {
            Format::Npy => sfs_core::sfs::io::Format::Npy,
            Format::Text => sfs_core::sfs::io::Format::Text,
        }
    }
}

impl View {
    pub fn run(self) -> Result<(), Error> {
        let mut sfs = sfs_core::sfs::io::read::Builder::default()
            .read_from_path_or_stdin(self.path.as_ref())?;

        // If marginalizing, normalize to indices to marginalize away (rather than keep)
        if let Some(axes) = match (self.marginalize_keep, self.marginalize_remove) {
            (Some(keep), None) => Some(
                (0..sfs.dimensions())
                    .filter(|i| !keep.contains(i))
                    .collect(),
            ),
            (None, Some(remove)) => Some(remove),
            (None, None) => None,
            (Some(_), Some(_)) => unreachable!("checked by clap"),
        } {
            let axes = axes.into_iter().map(Axis).collect::<Vec<_>>();
            sfs = sfs.marginalize(&axes)?;
        };

        sfs_core::sfs::io::write::Builder::default()
            .set_precision(self.precision)
            .set_format(sfs_core::sfs::io::Format::from(self.output_format))
            .write_to_path_or_stdout(self.output, &sfs)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use clap::error::ErrorKind as ClapErrorKind;

    use crate::tests::try_parse_subcmd;

    #[test]
    fn test_marginalize_keep_and_remove_conflict() {
        let result = try_parse_subcmd::<View>("sfs view -m 1 -M 2,3 input.sfs");

        assert_eq!(result.unwrap_err().kind(), ClapErrorKind::ArgumentConflict)
    }
}
