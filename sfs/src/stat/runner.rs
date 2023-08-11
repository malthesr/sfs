use std::{fmt, io};

use anyhow::{anyhow, Error};

use sfs_core::Scs;

use super::Statistic;

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
    scs: Scs,
    statistics: Vec<StatisticWithOptions>,
    header: bool,
    delimiter: char,
}

impl Runner<io::StdoutLock<'static>> {
    pub fn new(
        scs: Scs,
        statistics: Vec<StatisticWithOptions>,
        header: bool,
        delimiter: char,
    ) -> Self {
        Self {
            writer: io::stdout().lock(),
            scs,
            statistics,
            header,
            delimiter,
        }
    }
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
            .map(|s| match s.statistic.calculate(&self.scs) {
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
