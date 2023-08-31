use std::{fmt, path::PathBuf};

use anyhow::Error;

use clap::{CommandFactory, Parser, ValueEnum};
use sfs_core::{
    spectrum::{self, Scs},
    Input,
};

mod runner;
use runner::{Runner, StatisticWithOptions};

/// Calculate statistics from SFS.
#[derive(Debug, Parser)]
#[clap(name = crate::NAME, about)]
pub struct Stat {
    /// Input SFS.
    ///
    /// The input SFS can be provided here or read from stdin. The SFS will be normalised as
    /// required for particular statistics, so the input SFS does not need to be normalised.
    #[clap(value_parser, value_name = "PATH")]
    pub input: Option<PathBuf>,

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
    /// Fu and Li's D statistic. 1D SFS only. See Durrett (2008).
    DFuLi,
    /// Tajima's D statistic. 1D SFS only. See Durrett (2008).
    DTajima,
    /// The f₂-statistic. 2D SFS only. See Peter (2016).
    F2,
    /// The f₃(A; B, C)-statistic, where A, B, C is in the order of the populations in the SFS.
    /// 3D SFS only. See Peter (2016).
    F3,
    /// The f₄(A, B; C, D)-statistic, where A, B, C, D is in the order of the populations in the SFS.
    /// 4D SFS only. See Peter (2016).
    F4,
    /// Hudson's estimator of Fst, as ratio of averages. 2D SFS only.
    /// See Bhatia et al. (2013).
    Fst,
    /// Average pairwise differences. 1D SFS only.
    Pi,
    /// Average pairwise differences between two populations, also known as Dxy. 2D SFS only.
    /// See Nei and Li (1979), Cruickshank and Hahn (2014).
    PiXY,
    /// The King kinship statistic. Shape 3x3 2D SFS only. See Waples et al. (2019).
    King,
    /// The R0 kinship statistic. Shape 3x3 2D SFS only. See Waples et al. (2019).
    R0,
    /// The R1 kinship statistic. Shape 3x3 2D SFS only. See Waples et al. (2019).
    R1,
    /// Number of segregating sites (in at least one population).
    S,
    /// Sum of SFS. This will be total number of sites if not normalized, and ≈1 otherwise.
    Sum,
    /// Watterson's estimator of θ. 1D SFS only. Use π for Tajima's estimator. See Durrett (2008).
    Theta,
}

impl Statistic {
    pub fn calculate(self, scs: &Scs) -> Result<f64, Error> {
        Ok(match self {
            Statistic::DFuLi => scs.d_fu_li()?,
            Statistic::DTajima => scs.d_tajima()?,
            Statistic::F2 => scs.clone().into_normalized().f2()?,
            Statistic::F3 => scs.clone().into_normalized().f3()?,
            Statistic::F4 => scs.clone().into_normalized().f4()?,
            Statistic::Fst => scs.clone().into_normalized().fst()?,
            Statistic::King => scs.king()?,
            Statistic::Pi => scs.pi()?,
            Statistic::PiXY => scs.pi_xy()?,
            Statistic::R0 => scs.r0()?,
            Statistic::R1 => scs.r1()?,
            Statistic::S => scs.segregating_sites(),
            Statistic::Sum => scs.sum(),
            Statistic::Theta => scs.theta_watterson()?,
        })
    }

    pub fn header_name(&self) -> &'static str {
        match self {
            Statistic::DFuLi => "d_fu_li",
            Statistic::DTajima => "d_tajima",
            Statistic::F2 => "f2",
            Statistic::F3 => "f3",
            Statistic::F4 => "f4",
            Statistic::Fst => "fst",
            Statistic::King => "king",
            Statistic::Pi => "pi",
            Statistic::PiXY => "pi_xy",
            Statistic::R0 => "r0",
            Statistic::R1 => "r1",
            Statistic::S => "segregating_sites",
            Statistic::Sum => "sum",
            Statistic::Theta => "theta",
        }
    }
}

impl fmt::Display for Statistic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.header_name())
    }
}

impl Stat {
    pub fn run(self) -> Result<(), Error> {
        let scs = spectrum::io::read::Builder::default()
            .set_input(Input::new(self.input)?)
            .read()?;

        let statistics = match (&self.precision[..], &self.statistics[..]) {
            (&[precision], statistics) => statistics
                .iter()
                .map(|&s| StatisticWithOptions::new(s, precision))
                .collect::<Vec<_>>(),
            (precisions, statistics) if precisions.len() == statistics.len() => statistics
                .iter()
                .zip(precisions.iter())
                .map(|(&s, &p)| StatisticWithOptions::new(s, p))
                .collect::<Vec<_>>(),
            (precisions, statistics) => {
                return Err(Stat::command()
                    .error(
                        clap::error::ErrorKind::ValueValidation,
                        format!(
                            "number of precision specifiers must equal one \
                                or the number of statistics \
                                (found {} precision specifiers and {} statistics)",
                            precisions.len(),
                            statistics.len()
                        ),
                    )
                    .into());
            }
        };

        let mut runner = Runner::new(scs, statistics, self.header, self.delimiter);
        runner.run()
    }
}
