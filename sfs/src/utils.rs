use std::{
    env,
    io::{self, IsTerminal},
    path::Path,
};

use clap::CommandFactory;

use crate::Cli;

const ALLOW_STDIN_KEY: &str = "SFS_ALLOW_STDIN";

pub fn check_input_xor_stdin<P>(input: Option<P>) -> Result<(), clap::Error>
where
    P: AsRef<Path>,
{
    if input.is_some() && !io::stdin().is_terminal() && env::var(ALLOW_STDIN_KEY).is_err() {
        Err(Cli::command().error(
            clap::error::ErrorKind::TooManyValues,
            "received input both via file and stdin",
        ))
    } else if input.is_none() && std::io::stdin().is_terminal() {
        Err(Cli::command().error(
            clap::error::ErrorKind::TooFewValues,
            "received no input via file or stdin",
        ))
    } else {
        Ok(())
    }
}
