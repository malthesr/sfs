use std::path::PathBuf;

use anyhow::Error;

use clap::{Parser, ValueEnum};

use sfs_core::{spectrum, Input};

/// Fold SFS.
#[derive(Debug, Parser)]
#[clap(name = crate::NAME, about)]
pub struct Fold {
    /// Input SFS.
    ///
    /// The input SFS can be provided here or read from stdin in any of the supported formats.
    #[clap(value_parser, value_name = "PATH")]
    pub input: Option<PathBuf>,

    /// Fill value to use when folding.
    ///
    /// By default, the "lower" part of the SFS will be filled with nan values. Set this option to
    /// use another fill values.
    #[clap(short = 's', long, default_value = "nan", value_name = "FILL")]
    // default_value_t does not work well here since floats are formatted different from the Clap
    // enum string representation
    pub fill: Fill,

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
pub enum Fill {
    /// Set folded value to nan.
    Nan,
    /// Set folded value to 0.
    Zero,
    /// Set folded value to -1.
    MinusOne,
    /// Set folded value to Inf.
    Inf,
}

impl From<Fill> for f64 {
    fn from(value: Fill) -> Self {
        match value {
            Fill::Nan => f64::NAN,
            Fill::Zero => 0.,
            Fill::MinusOne => -1.,
            Fill::Inf => f64::INFINITY,
        }
    }
}

impl Fold {
    pub fn run(self) -> Result<(), Error> {
        let mut scs = spectrum::io::read::Builder::default()
            .set_input(Input::new(self.input)?)
            .read()?;

        scs = scs.fold().into_spectrum(f64::from(self.fill));

        spectrum::io::write::Builder::default()
            .set_precision(self.precision)
            .write_to_path_or_stdout(self.output, &scs)?;

        Ok(())
    }
}
