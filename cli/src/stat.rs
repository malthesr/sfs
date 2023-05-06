use std::{fmt, path::PathBuf};

use anyhow::Error;

use clap::{Parser, ValueEnum};
use sfs::{
    stat::{Fst, Heterozygosity, King, F2, R0, R1},
    Sfs,
};

mod runner;
use runner::Runner;

/// Calculate statistics from SFS.
#[derive(Debug, Parser)]
pub struct Stat {
    /// Input SFS.
    ///
    /// The input SFS can be provided here or read from stdin. The SFS will be normalised as
    /// required for particular statistics, so the input SFS does not need to be normalised.
    #[clap(value_parser, value_name = "PATH")]
    pub path: Option<PathBuf>,

    /// Delimiter between statistics.
    #[clap(short = 'd', long, default_value_t = ',', value_name = "CHAR")]
    pub delimiter: char,

    /// Include a header with the names of statistics.
    #[clap(short = 'H', long)]
    pub header: bool,

    /// Precision to use when printing statistics.
    ///
    /// If a single value is provided, this will be used for all statistics. If more than one
    /// statistic is calculated, the same number of precision specifiers may be provided, and they
    /// will be applied in the same order. Use comma to separate precision specifiers.
    #[clap(
        short = 'p',
        long,
        default_value = "6",
        use_value_delimiter = true,
        value_name = "INT,..."
    )]
    pub precision: Vec<usize>,

    /// Statistics to calculate.
    ///
    /// More than one statistic can be output. Use comma to separate statistics.
    /// An error will be thrown if the shape or dimensionality of the SFS is incompatible with
    /// the required statistics.
    #[clap(
        short = 's',
        long,
        value_enum,
        required = true,
        use_value_delimiter = true,
        value_name = "STAT,..."
    )]
    pub statistics: Vec<Statistic>,
}

#[derive(ValueEnum, Clone, Copy, Debug, Eq, PartialEq)]
pub enum Statistic {
    /// 2D SFS only. Based on all sites (including fixed), and may therefore have a
    /// different scaling factor than when based on SNPs.
    F2,
    /// 2D SFS only. Based on Hudson's estimate implemented as ratio of averages from
    /// Bhatia et al. (2013).
    Fst,
    /// Shape 3 1D SFS only.
    Heterozygosity,
    /// Shape 3x3 2D SFS only. Based on Waples et al. (2019).
    King,
    /// Shape 3x3 2D SFS only. Based on Waples et al. (2019).
    R0,
    /// Shape 3x3 2D SFS only. Based on Waples et al. (2019).
    R1,
    /// Sum of SFS.
    Sum,
}

impl Statistic {
    pub fn calculate(self, sfs: &Sfs) -> Result<f64, Error> {
        Ok(match self {
            Statistic::F2 => F2::from_sfs(&sfs.clone().into_normalized())?.0,
            Statistic::Fst => Fst::from_sfs(&sfs.clone().into_normalized())?.0,
            Statistic::Heterozygosity => {
                Heterozygosity::from_sfs(&sfs.clone().into_normalized())?.0
            }
            Statistic::King => King::from_sfs(sfs)?.0,
            Statistic::R0 => R0::from_sfs(sfs)?.0,
            Statistic::R1 => R1::from_sfs(sfs)?.0,
            Statistic::Sum => sfs.iter().sum(),
        })
    }

    pub fn name(&self) -> &'static str {
        match self {
            Statistic::F2 => "f2",
            Statistic::Fst => "fst",
            Statistic::Heterozygosity => "heterozygosity",
            Statistic::King => "king",
            Statistic::R0 => "r0",
            Statistic::R1 => "r1",
            Statistic::Sum => "sum",
        }
    }
}

impl fmt::Display for Statistic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

impl Stat {
    pub fn run(self) -> Result<(), Error> {
        let mut runner = Runner::try_from(&self)?;
        runner.run()
    }
}
