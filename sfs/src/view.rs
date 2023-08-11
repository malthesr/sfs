use std::{fmt, path::PathBuf};

use anyhow::Error;

use clap::{Args, Parser, ValueEnum};

use sfs_core::{
    array::{Axis, Shape},
    spectrum, Input,
};

/// Format, marginalize, project, and convert SFS.
///
/// Note that the order of operations matter: marginalization occurs before projection. To control
/// the order of operations differently, chain together multiple commands by piping in the desired
/// order.
#[derive(Debug, Parser)]
pub struct View {
    /// Input SFS.
    ///
    /// The input SFS can be provided here or read from stdin in any of the supported formats.
    #[clap(value_parser, value_name = "PATH")]
    pub input: Option<PathBuf>,

    /// Output path.
    ///
    /// If no path is given, SFS will be output to stdout.
    #[clap(short = 'o', long, value_name = "PATH")]
    pub output: Option<PathBuf>,

    /// Output format.
    #[clap(short = 'O', long, default_value_t = Format::Text, value_name = "FORMAT")]
    pub output_format: Format,

    #[command(flatten)]
    marginalize: Option<Marginalize>,

    #[command(flatten)]
    project: Option<Project>,

    /// Print precision.
    ///
    /// This is only used for printing SFS to plain text format, and will be ignored otherwise.
    #[clap(long, default_value_t = 6, value_name = "INT")]
    pub precision: usize,
}

#[derive(Args, Debug, Eq, PartialEq)]
#[group(required = false, multiple = false)]
struct Marginalize {
    /// Marginalize populations.
    ///
    /// Marginalize out provided populations. Marginalization corresponds to an
    /// array sum over the SFS seen as an array. Use a comma-separated list of 0-based dimensions to
    ///  keep, using the same ordering of the dimensions of the SFS as specified e.g. in the header.
    #[clap(
        short = 'm',
        long = "marginalize-remove",
        use_value_delimiter = true,
        value_name = "INT,..."
    )]
    pub remove: Option<Vec<usize>>,

    /// Marginalize remaining populations.
    ///
    /// Alternative to `--marginalize-remove`, see documentation for background. Using this
    /// argument, the marginalization can be specified in terms of dimensions to keep, rather than
    /// dimensions to remove.
    #[clap(
        short = 'M',
        long = "marginalize-keep",
        use_value_delimiter = true,
        value_name = "INT,..."
    )]
    pub keep: Option<Vec<usize>>,
}

#[derive(Args, Debug, Eq, PartialEq)]
#[group(required = false, multiple = false)]
struct Project {
    /// Projected individuals.
    ///
    /// Using this argument, it is possible to project the SFS down to a lower number of
    /// individuals.  Use a comma-separated list of values giving the new shape of the SFS.
    /// For example, `--project-individuals 3,2` would project a two-dimensional SFS down to three
    /// individuals in the first dimension and two in the second.
    ///
    /// Note that it is also possible to project during creation of the SFS using the `create`
    /// subcommand, and projection after creation is not in equivalent. Where applicable,
    /// prefer projecting during creation to use more data in the presence of missingness.
    #[clap(
        short = 'p',
        long = "project-individuals",
        use_value_delimiter = true,
        value_name = "INT,..."
    )]
    individuals: Option<Vec<usize>>,

    /// Projected shape.
    ///
    /// Alternative to `--project-individuals`, see documentation for background. Using this
    /// argument, the projection can be specified by shape, rather than number of individuals.
    /// For example, `--project-shape 7,5` would project a two-dimensional SFS down to three diploid
    /// individuals in the first dimension and two in the second.
    #[clap(
        long = "project-shape",
        use_value_delimiter = true,
        value_name = "INT,..."
    )]
    shape: Option<Vec<usize>>,
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

impl From<Format> for sfs_core::spectrum::io::Format {
    fn from(value: Format) -> Self {
        match value {
            Format::Npy => sfs_core::spectrum::io::Format::Npy,
            Format::Text => sfs_core::spectrum::io::Format::Text,
        }
    }
}

impl View {
    pub fn run(self) -> Result<(), Error> {
        let mut scs = spectrum::io::read::Builder::default()
            .set_input(Input::new(self.input)?)
            .read()?;

        if let Some(marginalize) = self.marginalize {
            // If marginalizing, normalize to indices to marginalize away (rather than keep)
            let axes = match (marginalize.keep, marginalize.remove) {
                (Some(keep), None) => (0..scs.dimensions())
                    .filter(|i| !keep.contains(i))
                    .collect(),

                (None, Some(remove)) => remove,
                _ => unreachable!("checked by clap"),
            };

            let axes = axes.into_iter().map(Axis).collect::<Vec<_>>();
            scs = scs.marginalize(&axes)?;
        }

        if let Some(project) = self.project {
            let shape = match (project.individuals, project.shape) {
                (Some(individuals), None) => {
                    Shape(individuals.into_iter().map(|i| 2 * i + 1).collect())
                }
                (None, Some(shape)) => Shape(shape),
                _ => unreachable!("checked by clap"),
            };

            scs = scs.project(shape)?;
        }

        spectrum::io::write::Builder::default()
            .set_precision(self.precision)
            .set_format(sfs_core::spectrum::io::Format::from(self.output_format))
            .write_to_path_or_stdout(self.output, &scs)?;

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
