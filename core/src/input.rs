//! Input for creating spectra.

use std::{
    env,
    fs::File,
    io::{self, IsTerminal as _},
    path::{Path, PathBuf},
};

pub mod genotype;
pub use genotype::Genotype;

pub mod sample;
pub use sample::Sample;

pub mod site;
pub use site::Site;

/// A status when trying to read an element from a reader.
#[derive(Debug)]
pub enum ReadStatus<T> {
    /// Element was succesfully read.
    Read(T),
    /// An error was encountered.
    Error(io::Error),
    /// The reader has finished.
    Done,
}

impl<T> ReadStatus<T> {
    fn map<U, F>(self, op: F) -> ReadStatus<U>
    where
        F: FnOnce(T) -> U,
    {
        match self {
            ReadStatus::Read(t) => ReadStatus::Read(op(t)),
            ReadStatus::Error(e) => ReadStatus::Error(e),
            ReadStatus::Done => ReadStatus::Done,
        }
    }
}

/// An input source for reading.
#[derive(Debug)]
pub enum Input {
    /// A path from which to read a file.
    Path(PathBuf),
    /// Stdin.
    Stdin,
}

impl Input {
    /// By default, reading an `Input` checks that either a path is provided, or that input is
    /// available via stdin, instead of hanging.
    ///
    /// In some contexts, e.g. testing, this can cause issues, and so it may be disabled by setting
    /// this environment variable, or by using [`Input::new_unchecked`].
    pub const ENV_KEY_DISABLE_CHECK: &'static str = "SFS_ALLOW_STDIN";

    /// Creates a new input source.
    pub fn new(input: Option<PathBuf>) -> io::Result<Self> {
        let check = env::var(Self::ENV_KEY_DISABLE_CHECK).is_err();

        if input.is_some() && !io::stdin().is_terminal() && check {
            Err(io::Error::new(
                io::ErrorKind::Other,
                "received input both via file and stdin",
            ))
        } else if input.is_none() && std::io::stdin().is_terminal() && check {
            Err(io::Error::new(
                io::ErrorKind::Other,
                "received no input via file or stdin",
            ))
        } else {
            Ok(Self::new_unchecked(input))
        }
    }

    /// Creates a new input source without checking that any data is available.
    pub fn new_unchecked(input: Option<PathBuf>) -> Self {
        if let Some(path) = input {
            Self::Path(path)
        } else {
            Self::Stdin
        }
    }

    /// Open the input for reading.
    pub fn open(&self) -> io::Result<Reader> {
        match self {
            Input::Path(path) => File::open(path).map(io::BufReader::new).map(Reader::File),
            Input::Stdin => Ok(Reader::Stdin(io::stdin().lock())),
        }
    }

    /// Returns the provided path if provided, otherwise `None`.
    pub fn as_path(&self) -> Option<&Path> {
        match self {
            Input::Path(path) => Some(path.as_ref()),
            Input::Stdin => None,
        }
    }
}

impl From<Input> for Option<PathBuf> {
    fn from(input: Input) -> Self {
        match input {
            Input::Path(path) => Some(path),
            Input::Stdin => None,
        }
    }
}

/// A reader from either a file or stdin.
#[derive(Debug)]
pub enum Reader {
    /// A reader from a file.
    File(io::BufReader<File>),
    /// A reader stdin.
    Stdin(io::StdinLock<'static>),
}
