use super::{
    shape::{RemovedAxis, Strides},
    Array, Shape,
};

mod iter;
pub use iter::Iter;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct View<'a> {
    pub(crate) data: &'a [f64],
    pub(crate) offset: usize,
    pub(crate) shape: RemovedAxis<'a, Shape>,
    pub(crate) strides: RemovedAxis<'a, Strides>,
}

impl<'a> View<'a> {
    pub fn iter(&self) -> Iter<'_> {
        Iter::new(*self)
    }

    pub(crate) fn new_unchecked(
        data: &'a [f64],
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

    pub fn to_array(&self) -> Array {
        let data = self.iter().copied().collect();
        let shape = Shape(self.shape.iter().copied().collect());

        Array::new_unchecked(data, shape)
    }
}
