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

#[derive(Debug)]
pub enum ReadStatus<T> {
    Read(T),
    Error(io::Error),
    Done,
}

impl<T> ReadStatus<T> {
    pub fn map<U, F>(self, op: F) -> ReadStatus<U>
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

#[derive(Debug)]
pub enum Input {
    Path(PathBuf),
    Stdin,
}

impl Input {
    pub const ENV_KEY_DISABLE_CHECK: &'static str = "SFS_ALLOW_STDIN";

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

    pub fn new_unchecked(input: Option<PathBuf>) -> Self {
        if let Some(path) = input {
            Self::Path(path)
        } else {
            Self::Stdin
        }
    }

    pub fn open(&self) -> io::Result<Reader> {
        match self {
            Input::Path(path) => File::open(path).map(io::BufReader::new).map(Reader::File),
            Input::Stdin => Ok(Reader::Stdin(io::stdin().lock())),
        }
    }

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

#[derive(Debug)]
pub enum Reader {
    File(io::BufReader<File>),
    Stdin(io::StdinLock<'static>),
}
