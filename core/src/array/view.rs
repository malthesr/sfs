use super::{
    shape::{RemovedAxis, Strides},
    Array, Shape,
};

mod iter;
pub use iter::Iter;

#[derive(Debug, PartialEq)]
pub struct View<'a, T> {
    data: &'a [T], // first element is first element in view
    shape: RemovedAxis<'a, Shape>,
    strides: RemovedAxis<'a, Strides>,
}

impl<'a, T> Clone for View<'a, T> {
    fn clone(&self) -> Self {
        Self {
            data: self.data,
            shape: self.shape,
            strides: self.strides,
        }
    }
}

impl<'a, T> Copy for View<'a, T> {}

impl<'a, T> View<'a, T> {
    pub fn dimensions(&self) -> usize {
        self.shape.len()
    }

    pub fn iter(&self) -> Iter<'_, T> {
        Iter::new(*self)
    }

    pub(crate) fn new_unchecked(
        data: &'a [T],
        shape: RemovedAxis<'a, Shape>,
        strides: RemovedAxis<'a, Strides>,
    ) -> Self {
        Self {
            data,
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
