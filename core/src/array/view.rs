use super::{
    shape::{RemovedAxis, Strides},
    Array, Shape,
};

mod iter;
pub use iter::Iter;

#[derive(Debug, PartialEq)]
pub struct View<'a, T> {
    data: &'a [T],
    offset: usize,
    shape: RemovedAxis<'a, Shape>,
    strides: RemovedAxis<'a, Strides>,
}

impl<'a, T> Clone for View<'a, T> {
    fn clone(&self) -> Self {
        Self {
            data: self.data,
            offset: self.offset,
            shape: self.shape,
            strides: self.strides,
        }
    }
}

impl<'a, T> Copy for View<'a, T> {}

impl<'a, T> View<'a, T> {
    pub fn iter(&self) -> Iter<'_, T> {
        Iter::new(*self)
    }

    pub(crate) fn new_unchecked(
        data: &'a [T],
        offset: usize,
        shape: RemovedAxis<'a, Shape>,
        strides: RemovedAxis<'a, Strides>,
    ) -> Self {
        Self {
            data,
            offset,
            shape,
            strides,
        }
    }

    pub fn to_array(&self) -> Array<T>
    where
        T: Clone,
    {
        let data = Vec::from_iter(self.iter().cloned());
        let shape = Shape(self.shape.iter().copied().collect());

        Array::new_unchecked(data, shape)
    }
}
