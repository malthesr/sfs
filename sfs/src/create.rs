use std::{io, num::NonZeroUsize, path::PathBuf};

use anyhow::Error;

use clap::{Args, Parser};

mod runner;
use runner::Runner;
use sfs_core::reader::site;

use crate::utils::check_input_xor_stdin;

/// Create SFS from VCF/BCF.
#[derive(Debug, Parser)]
pub struct Create {
    /// Input VCF/BCF.
    ///
    /// If no file is provided, stdin will be used. Input may be BGZF-compressed or uncompressed.
    #[arg(value_name = "FILE")]
    input: Option<PathBuf>,

    /// Output precision.
    ///
    /// This option is only used when projecting, and otherwise set to zero since the output must be
    /// integer counts.
    #[arg(long, default_value_t = 6, value_name = "INT")]
    precision: usize,

    #[command(flatten)]
    project: Option<Project>,

    #[command(flatten)]
    samples: Option<Samples>,

    /// Fail on missingness.
    ///
    /// By default, any site with missing and/or multiallelic genotypes in the applied sample subset
    /// are skipped and logged. Using this flag will cause an error if such genotypes are
    /// encountered.
    #[arg(long)]
    strict: bool,

    /// Number of threads.
    ///
    /// Multi-threading currently only affects reading and parsing BGZF compressed input.
    #[arg(short = 't', long, default_value_t = NonZeroUsize::new(4).unwrap(), value_name = "INT")]
    threads: NonZeroUsize,
}

#[derive(Args, Debug, Eq, PartialEq)]
#[group(required = false, multiple = false)]
struct Samples {
    /// Sample subset.
    ///
    /// By default, a one-dimensional SFS of all samples is created. Using this argument, the subset
    /// of samples can be restricted. Multiple, comma-separated values may be provided. To construct
    /// a multi-dimensional SFS, the samples may be provided as `sample=population` pairs.
    /// The ordering of populations in the resulting SFS corresponds to the order of appearance of
    /// input population names.
    #[arg(
        short = 's',
        long = "samples",
        use_value_delimiter = true,
        value_delimiter = ',',
        value_parser = parse_key_val,
        value_name = "SAMPLE[=GROUP],...")
    ]
    list: Option<Vec<(String, Option<String>)>>,

    /// Sample subset file.
    ///
    /// Alternative to `--samples`, see documentation for background. Using this argument, the
    /// sample subset can be provided as a file. Each line should contain the name of a sample.
    /// Optionally, the file may contain a second, tab-delimited column with population identifiers.
    #[arg(short = 'S', long = "samples-file", value_name = "FILE")]
    file: Option<PathBuf>,
}

#[derive(Args, Debug, Eq, PartialEq)]
#[group(required = false, multiple = false, conflicts_with = "strict")]
struct Project {
    /// Projected individuals.
    ///
    /// By default, any site with missing and/or multiallelic genotypes in the applied sample subset
    /// will be skipped. Where this leads to too much missingness, the SFS can be projected to a
    /// lower number of individuals using hypergeometric sampling. By doing so, all sites with data
    /// for at least as this required shape will be used, and those with more data will be projected
    /// down. Use a comma-separated list of values giving the new shape of the SFS. For example,
    /// `--project-individuals 3,2` would project a two-dimensional SFS down to three individuals
    /// in the first dimension and two in the second.
    #[clap(
        short = 'p',
        long = "project-individuals",
        use_value_delimiter = true,
        value_name = "INT,..."
    )]
    individuals: Option<Vec<usize>>,

    /// Projected shape.
    ///
    /// Alternative to `--project-individuals`, see documentation for background. Using this argument, the
    /// projection can be specified by shape, rather than number of individuals. For example,
    /// `--project-shape 7,5` would project a two-dimensional SFS down to three diploid individuals
    /// in the first dimension and two in the second.
    #[clap(
        long = "project-shape",
        use_value_delimiter = true,
        value_name = "INT,..."
    )]
    shape: Option<Vec<usize>>,
}

fn parse_key_val(s: &str) -> Result<(String, Option<String>), clap::Error> {
    Ok(s.split_once('=')
        .map(|(key, val)| (key.to_string(), Some(val.to_string())))
        .unwrap_or_else(|| (s.to_string(), None)))
}

impl Create {
    pub fn run(self) -> Result<(), Error> {
        check_input_xor_stdin(self.input.as_ref())?;

        let mut builder = site::reader::Builder::default().set_threads(self.threads);

        if let Some(path) = self.input.to_owned() {
            builder = builder.set_path(path);
        }

        if let Some(samples) = self.samples {
            builder = match (samples.list, samples.file) {
                (Some(list), None) => builder.set_samples(list)?,
                (None, Some(file)) => builder.set_samples_file(file)?,
                _ => unreachable!("checked by clap"),
            };
        }

        let mut precision = 0;

        if let Some(project) = self.project {
            precision = self.precision;

            builder = match (project.individuals, project.shape) {
                (Some(individuals), None) => builder.set_project_individuals(individuals),
                (None, Some(shape)) => builder.set_project_shape(shape),
                _ => unreachable!("checked by clap"),
            };
        }

        let reader = builder.build()?;

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
            args.samples.and_then(|samples| samples.list),
            Some(vec![
                (String::from("sample0"), Some(String::from("group0"))),
                (String::from("sample1"), None),
                (String::from("sample2"), Some(String::from("group2"))),
            ])
        );
    }

    #[test]
    fn test_project() {
        let args = parse_subcmd::<Create>("sfs create --project-shape 6,3,9 input.bcf");

        assert_eq!(
            args.project.and_then(|project| project.shape),
            Some(vec![6, 3, 9])
        );
    }

    #[test]
    fn test_project_args_conflict() {
        let result = try_parse_subcmd::<Create>(
            "sfs create --project-shape 5 --project-individuals 2 input.bcf",
        );

        assert_eq!(result.unwrap_err().kind(), ClapErrorKind::ArgumentConflict)
    }

    #[test]
    fn test_project_strict_conflict() {
        let result = try_parse_subcmd::<Create>("sfs create -p 2 --strict input.bcf");

        assert_eq!(result.unwrap_err().kind(), ClapErrorKind::ArgumentConflict)
    }
}
