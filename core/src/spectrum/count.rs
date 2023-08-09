use std::ops::{Deref, Index, IndexMut};

use crate::array::Shape;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Count(pub Vec<usize>);

impl Count {
    pub fn dimensions(&self) -> usize {
        self.0.len()
    }

    pub fn from_shape(shape: Shape) -> Self {
        let mut vec = shape.0;
        vec.iter_mut().for_each(|x| *x -= 1);
        Self(vec)
    }

    pub fn from_zeros(dimensions: usize) -> Self {
        Self(vec![0; dimensions])
    }

    pub fn into_shape(self) -> Shape {
        let mut vec = self.0;
        vec.iter_mut().for_each(|x| *x += 1);
        Shape(vec)
    }

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