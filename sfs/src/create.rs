use std::{io, num::NonZeroUsize, path::PathBuf};

use anyhow::Error;

use clap::Parser;

mod runner;
use runner::Runner;
use sfs_core::reader;

/// Create SFS from BCF.
#[derive(Debug, Parser)]
pub struct Create {
    /// Input BCF file.
    ///
    /// If no file is provided, stdin will be used.
    #[arg(value_name = "FILE")]
    input: Option<PathBuf>,

    /// Output SFS precision.
    ///
    /// This option is only used when projecting, otherwise the output precision is 0.
    #[arg(long, default_value_t = 6, value_name = "INT")]
    precision: usize,

    /// Sample subset to use.
    ///
    /// By default, a 1-dimensional SFS of all samples is created. By providing a sample subset, the
    /// number of individuals considered can be restricted. Multiple, comma-separated values can be
    /// provided. To construct a multi-dimensional SFS, the samples may be provided as a
    /// comma-separated list of 'sample=group' pairs.
    #[arg(
        short = 's',
        long,
        use_value_delimiter = true,
        value_delimiter = ',',
        value_parser = parse_key_val,
        value_name = "SAMPLE[=GROUP],...")
    ]
    samples: Option<Vec<(String, Option<String>)>>,

    /// Samples file.
    ///
    /// By default, a 1-dimensional SFS of all samples is created. By providing a samples file, the
    /// number of individuals considered can be restricted. Each line should contain the name of a
    /// single sample in the input file. To construct a multi-dimensional SFS, the file may
    /// optionally contain a second, tab-delimited column contain group identifiers.
    #[arg(short = 'S', long, conflicts_with = "samples", value_name = "FILE")]
    samples_file: Option<PathBuf>,

    /// Hypergeometric projection of the SFS.
    ///
    /// By default, any site with missing and/or multiallelic genotypes are skipped. If this leads
    /// to an unacceptable amount of skipped sites, the SFS can be projected to a lower shape, by
    /// hypergeometric sampling. Use a comma-separated list of values giving the new shape of the
    /// SFS. For example, `--project 7,5` would project a two-dimensional SFS down to three diploid
    /// individuals in the first dimension and two in the second.
    #[clap(short = 'p', long, use_value_delimiter = true, value_name = "INT,...")]
    pub project: Option<Vec<usize>>,

    /// Promote warnings to errors.
    ///
    /// By default, missing and multiallelic genotypes will be skipped and logged. Using this flag
    /// will cause an error instead of a warning if such
    /// genotypes are encountered.
    #[arg(long)]
    strict: bool,

    /// Number of threads to use.
    #[arg(short = 't', long, default_value_t = NonZeroUsize::new(4).unwrap(), value_name = "INT")]
    threads: NonZeroUsize,
}

fn parse_key_val(s: &str) -> Result<(String, Option<String>), clap::Error> {
    Ok(s.split_once('=')
        .map(|(key, val)| (key.to_string(), Some(val.to_string())))
        .unwrap_or_else(|| (s.to_string(), None)))
}

impl Create {
    pub fn run(self) -> Result<(), Error> {
        let mut builder = reader::Builder::default().set_threads(self.threads);

        builder = if let Some(samples_file) = self.samples_file {
            builder.set_samples_file(samples_file)?
        } else if let Some(samples) = self.samples {
            builder.set_samples(samples)?
        } else {
            builder
        };

        let precision = self.project.as_ref().map(|_| self.precision).unwrap_or(0);

        if let Some(projection) = self.project {
            builder = builder.set_projection(projection.into());
        };

        let reader = builder.build_from_path_or_stdin(self.input.as_ref())?;

        let mut runner = Runner::new(reader, self.strict)?;
        let sfs = runner.run()?;

        sfs_core::spectrum::io::text::write_spectrum(&mut io::stdout(), &sfs, precision)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use clap::error::ErrorKind as ClapErrorKind;

    use crate::tests::{parse_subcmd, try_parse_subcmd};

    #[test]
    fn test_samples_and_samples_file_conflict() {
        let result = try_parse_subcmd::<Create>("sfs create -s sample0 -S samples.file input.bcf");

        assert_eq!(result.unwrap_err().kind(), ClapErrorKind::ArgumentConflict)
    }

    #[test]
    fn test_parse_samples() {
        let args =
            parse_subcmd::<Create>("sfs create -s sample0=group0,sample1,sample2=group2 input.bcf");

        assert_eq!(
            args.samples,
            Some(vec![
                (String::from("sample0"), Some(String::from("group0"))),
                (String::from("sample1"), None),
                (String::from("sample2"), Some(String::from("group2"))),
            ])
        );
    }

    #[test]
    fn test_projection() {
        let args = parse_subcmd::<Create>("sfs create -p 6,3,9 input.bcf");

        assert_eq!(args.project, Some(vec![6, 3, 9]));
    }
}
