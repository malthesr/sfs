use std::ops::{Deref, Index, IndexMut};

use crate::array::Shape;

/// An allele count.
///
/// This corresponds to an index in a [`Spectrum`].
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Count(pub Vec<usize>);

impl Count {
    /// The number of dimensions of the count.
    pub fn dimensions(&self) -> usize {
        self.0.len()
    }

    pub(crate) fn try_from_shape(shape: Shape) -> Option<Self> {
        let mut vec = shape.0;
        for x in vec.iter_mut() {
            *x = x.checked_sub(1)?;
        }
        Some(Self(vec))
    }

    /// Creates a new count from zeros.
    pub fn from_zeros(dimensions: usize) -> Self {
        Self(vec![0; dimensions])
    }

    pub(crate) fn into_shape(self) -> Shape {
        let mut vec = self.0;
        vec.iter_mut().for_each(|x| *x += 1);
        Shape(vec)
    }

    /// Set all elements to zero.
    pub fn set_zero(&mut self) {
        self.0.iter_mut().for_each(|x| *x = 0);
    }
}

impl AsRef<[usize]> for Count {
    fn as_ref(&self) -> &[usize] {
        self
    }
}

impl Deref for Count {
    type Target = [usize];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<Vec<usize>> for Count {
    fn from(shape: Vec<usize>) -> Self {
        Self(shape)
    }
}

impl<const N: usize> From<[usize; N]> for Count {
    fn from(shape: [usize; N]) -> Self {
        Self(shape.to_vec())
    }
}

impl From<usize> for Count {
    fn from(shape: usize) -> Self {
        Self(vec![shape])
    }
}

impl Index<usize> for Count {
    type Output = usize;

    fn index(&self, index: usize) -> &Self::Output {
        self.0.index(index)
    }
}

impl IndexMut<usize> for Count {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        self.0.index_mut(index)
    }
}
