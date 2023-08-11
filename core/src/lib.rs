#![deny(unsafe_code)]

#[cfg(test)]
#[macro_use]
pub(crate) mod approx;

pub mod input;

pub mod spectrum;
pub use spectrum::{Scs, Sfs, Spectrum};

pub mod array;
pub use array::Array;
