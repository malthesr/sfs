use std::path::PathBuf;

use anyhow::Error;

use clap::{Parser, ValueEnum};

/// Fold SFS.
#[derive(Debug, Parser)]
pub struct Fold {
    /// Input SFS.
    ///
    /// The input SFS can be provided here or read from stdin in any of the supported formats.
    #[clap(value_parser, value_name = "PATH")]
    pub path: Option<PathBuf>,

    /// Sentry value to use when folding.
    ///
    /// By default, the "lower" part of the SFS will be set to nan. Setting this option can change
    /// this to other sentry values.
    #[clap(short = 's', long, default_value = "nan", value_name = "SENTRY")]
    // default_value_t does not work well here since floats are formatted different from the Clap
    // enum string representation
    pub sentry: Sentry,

    /// Output SFS path.
    ///
    /// If no path is given, SFS will be output to stdout.
    #[clap(short = 'o', long, value_name = "PATH")]
    pub output: Option<PathBuf>,

    /// Precision to use when printing SFS.
    #[clap(short = 'p', long, default_value_t = 6, value_name = "INT")]
    pub precision: usize,
}

#[derive(ValueEnum, Clone, Copy, Debug, Eq, PartialEq)]
pub enum Sentry {
    /// Set folded value to nan.
    Nan,
    /// Set folded value to 0.
    Zero,
    /// Set folded value to -1.
    MinusOne,
    /// Set folded value to Inf.
    Inf,
}

impl From<Sentry> for f64 {
    fn from(value: Sentry) -> Self {
        match value {
            Sentry::Nan => f64::NAN,
            Sentry::Zero => 0.,
            Sentry::MinusOne => -1.,
            Sentry::Inf => f64::INFINITY,
        }
    }
}

impl Fold {
    pub fn run(self) -> Result<(), Error> {
        let mut sfs =
            sfs::io::read::Builder::default().read_from_path_or_stdin(self.path.as_ref())?;

        sfs = sfs.fold(f64::from(self.sentry));

        sfs::io::write::Builder::default()
            .set_precision(self.precision)
            .write_to_path_or_stdout(self.output, &sfs)?;

        Ok(())
    }
}
