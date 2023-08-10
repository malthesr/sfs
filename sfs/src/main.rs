#![deny(unsafe_code)]

use std::io::Write;

use anyhow::Error;

use clap::{ArgAction, Parser, Subcommand};

mod create;
use create::Create;

mod fold;
use fold::Fold;

mod stat;
use stat::Stat;

mod view;
use view::View;

pub(crate) mod utils;

const NAME: &str = env!("CARGO_BIN_NAME");
const VERSION: &str = env!("CARGO_PKG_VERSION");
const AUTHOR: &str = env!("CARGO_PKG_AUTHORS");

/// Tools for working with site frequency spectra.
#[derive(Debug, Parser)]
#[clap(name = NAME, author = AUTHOR, version = VERSION, about)]
#[clap(subcommand_required = true)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    command: Command,

    /// Suppress log output.
    ///
    /// By default, information may be logged to stderr while running. Set this flag once to silence
    /// normal logging output, and set twice to silence warnings.
    #[arg(
        short = 'q',
        long,
        action = ArgAction::Count,
        global = true,
        conflicts_with = "verbose"
    )]
    quiet: u8,

    /// Log output verbosity.
    ///
    /// Set this flag times to show debug information, and set twice to show trace information.
    #[clap(short = 'v', long, action = ArgAction::Count, global = true)]
    verbose: u8,

    /// Print CLI arguments for debugging.
    #[clap(long, hide = true, global = true)]
    debug: bool,
}

impl Cli {
    pub fn run(self) -> Result<(), Error> {
        if self.debug {
            eprintln!("{self:#?}");
        }

        let level = if self.quiet > 0 {
            match self.quiet {
                1 => log::LevelFilter::Warn,
                _ => log::LevelFilter::Off,
            }
        } else {
            match self.verbose {
                0 => log::LevelFilter::Info,
                1 => log::LevelFilter::Debug,
                _ => log::LevelFilter::Trace,
            }
        };

        match env_logger::Builder::new()
            .filter_level(level)
            .target(env_logger::Target::Stderr)
            .format(|buf, record| {
                let level = record.level().as_str().to_lowercase();
                let args = record.args();
                writeln!(buf, "[sfs {level:>5}] {args}")
            })
            .try_init()
        {
            Ok(()) => (),
            Err(e) => eprintln!("failed to setup logger: {e}"),
        }

        self.command.run()
    }
}

#[derive(Debug, Subcommand)]
pub enum Command {
    Create(Create),
    Fold(Fold),
    Stat(Stat),
    View(View),
}

impl Command {
    fn run(self) -> Result<(), Error> {
        match self {
            Command::Create(create) => create.run(),
            Command::Fold(fold) => fold.run(),
            Command::Stat(stat) => stat.run(),
            Command::View(view) => view.run(),
        }
    }
}

impl TryFrom<Command> for Create {
    type Error = Command;

    fn try_from(command: Command) -> Result<Self, Self::Error> {
        match command {
            Command::Create(create) => Ok(create),
            _ => Err(command),
        }
    }
}

impl TryFrom<Command> for Fold {
    type Error = Command;

    fn try_from(command: Command) -> Result<Self, Self::Error> {
        match command {
            Command::Fold(fold) => Ok(fold),
            _ => Err(command),
        }
    }
}

impl TryFrom<Command> for Stat {
    type Error = Command;

    fn try_from(command: Command) -> Result<Self, Self::Error> {
        match command {
            Command::Stat(stat) => Ok(stat),
            _ => Err(command),
        }
    }
}

impl TryFrom<Command> for View {
    type Error = Command;

    fn try_from(command: Command) -> Result<Self, Self::Error> {
        match command {
            Command::View(view) => Ok(view),
            _ => Err(command),
        }
    }
}

fn main() {
    let cli = Cli::parse();

    match cli.run() {
        Ok(()) => (),
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use clap::error::ErrorKind as ClapErrorKind;

    fn try_parse_args(cmd: &str) -> Result<Cli, clap::Error> {
        Parser::try_parse_from(cmd.split_whitespace())
    }

    pub fn try_parse_subcmd<T>(cmd: &str) -> Result<T, clap::Error>
    where
        T: TryFrom<Command>,
        T::Error: std::fmt::Debug,
    {
        try_parse_args(cmd).map(|cli| T::try_from(cli.command).expect("wrong subcommand"))
    }

    pub fn parse_subcmd<T>(cmd: &str) -> T
    where
        T: TryFrom<Command>,
        T::Error: std::fmt::Debug,
    {
        try_parse_subcmd(cmd).expect("failed to parse command")
    }

    #[test]
    fn test_no_subcommand() {
        let result = try_parse_args("sfs");

        assert_eq!(
            result.unwrap_err().kind(),
            ClapErrorKind::DisplayHelpOnMissingArgumentOrSubcommand
        );
    }
}
