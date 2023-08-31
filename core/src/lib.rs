#![deny(unsafe_code)]
#![warn(missing_docs)]

//! Tools for working with site frequency spectra.
//!
//! This serves as the core library implementation for the `sfs` CLI, but can also be used as a
//! free-standing library for working with frequency spectra.
//!
//! # Overview
//!
//! The core struct is a [`Spectrum`], which is backed by an N-dimensional [`Array`].
//! A spectrum may either be a *frequency* spectrum ([`Sfs`]), in which case we say that it is
//! normalized, or it may be a *count* spectrum ([`Scs`]).
//!
//! # Example
//!
//! As a very brief introduction to the API, let's create a count spectrum, and then normalize it
//! to obtain a per-base estimate of θ using Watterson's estimator.
//!
//! ```
//! use sfs_core::Scs;
//!
//! // Create a 1-dimensional SCS from some data
//! let scs = Scs::from_vec([25., 8., 4., 2., 1., 0.]);
//!
//! // Normalize the spectrum to frequencies
//! let sfs = scs.into_normalized();
//!
//! // Calculate θ
//! let theta = sfs.theta_watterson().expect("θ only defined in 1D");
//!
//! assert!((theta - 0.18).abs() < 1e-16);
//! ```

#[cfg(test)]
#[macro_use]
pub(crate) mod approx;

pub mod input;
pub use input::Input;

pub mod spectrum;
pub use spectrum::{Scs, Sfs, Spectrum};

pub mod array;
pub use array::Array;

pub mod utils;
