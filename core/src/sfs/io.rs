//! Utilities for reading and writing SFS.

pub mod read;
pub mod text;
pub mod write;

use crate::array::npy;

/// Supported SFS formats.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Format {
    // Numpy binary npy format.
    Npy,
    // Plain text format.
    Text,
}

impl Format {
    fn detect(bytes: &[u8]) -> Option<Self> {
        Self::detect_npy(bytes).xor(Self::detect_plain_text(bytes))
    }

    fn detect_npy(bytes: &[u8]) -> Option<Self> {
        (bytes[..npy::MAGIC.len()] == npy::MAGIC).then_some(Self::Npy)
    }

    fn detect_plain_text(bytes: &[u8]) -> Option<Self> {
        (bytes[..text::START.len()] == text::START).then_some(Self::Text)
    }
}
