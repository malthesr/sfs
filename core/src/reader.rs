use std::io;

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
