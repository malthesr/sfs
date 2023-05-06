use std::{fmt, io};

use anyhow::{anyhow, Error};

use sfs::Sfs;

use super::{Stat, Statistic};

#[derive(Clone, Debug, PartialEq)]
pub struct StatisticWithOptions {
    statistic: Statistic,
    precision: usize,
}

impl StatisticWithOptions {
    pub fn new(statistic: Statistic, precision: usize) -> Self {
        Self {
            statistic,
            precision,
        }
    }
}

#[derive(Debug)]
pub struct Runner<W> {
    writer: W,
    sfs: Sfs,
    statistics: Vec<StatisticWithOptions>,
    header: bool,
    delimiter: char,
}

impl<W> Runner<W>
where
    W: io::Write,
{
    pub fn run(&mut self) -> Result<(), Error> {
        if self.header {
            self.write_header()?;
        }

        self.write_statistics()
    }

    fn write_header(&mut self) -> Result<(), Error> {
        let header_names = self
            .statistics
            .iter()
            .map(|s| s.statistic.name())
            .collect::<Vec<_>>();

        self.write_with_delimiter(header_names)
    }

    fn write_statistics(&mut self) -> Result<(), Error> {
        let statistics = self
            .statistics
            .iter()
            .map(|s| match s.statistic.calculate(&self.sfs) {
                Ok(stat) => Ok(format!("{stat:.precision$}", precision = s.precision)),
                Err(e) => Err(anyhow!(e)),
            })
            .collect::<Result<Vec<_>, _>>()?;

        self.write_with_delimiter(statistics)
    }

    fn write_with_delimiter<I>(&mut self, items: I) -> Result<(), Error>
    where
        I: IntoIterator,
        I::Item: fmt::Display,
    {
        for (i, x) in items.into_iter().enumerate() {
            if i > 0 {
                write!(self.writer, "{}", self.delimiter)?;
            }
            write!(self.writer, "{x}")?;
        }
        writeln!(self.writer)?;

        Ok(())
    }
}

impl TryFrom<&Stat> for Runner<io::StdoutLock<'static>> {
    type Error = Error;

    fn try_from(args: &Stat) -> Result<Self, Self::Error> {
        let sfs = sfs::io::read::Builder::default().read_from_path_or_stdin(args.path.as_ref())?;

        let statistics = match (&args.precision[..], &args.statistics[..]) {
            (&[precision], statistics) => statistics
                .iter()
                .map(|&s| StatisticWithOptions::new(s, precision))
                .collect::<Vec<_>>(),
            (precisions, statistics) if precisions.len() == statistics.len() => statistics
                .iter()
                .zip(precisions.iter())
                .map(|(&s, &p)| StatisticWithOptions::new(s, p))
                .collect::<Vec<_>>(),
            (precisions, statistics) => Err(anyhow!(
                "number of precision specifiers must equal one \
                    or the number of statistics \
                    (found {} precision specifiers and {} statistics)",
                precisions.len(),
                statistics.len()
            ))?,
        };

        Ok(Self {
            writer: io::stdout().lock(),
            sfs,
            statistics,
            header: args.header,
            delimiter: args.delimiter,
        })
    }
}
