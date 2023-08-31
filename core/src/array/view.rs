//! Array views.

use super::{
    shape::{RemovedAxis, Strides},
    Array, Shape,
};

mod iter;
pub use iter::Iter;

/// A view of an array along a particular axis.
///
/// See [`Array::get_axis`], [`Array::index_axis`], and [`Array::iter_axis`] for methods to obtain
/// axis views.
#[derive(Debug, PartialEq)]
pub struct View<'a, T> {
    data: &'a [T], // first element is first element in view
    shape: RemovedAxis<'a, Shape>,
    strides: RemovedAxis<'a, Strides>,
}

impl<'a, T> Clone for View<'a, T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<'a, T> Copy for View<'a, T> {}

impl<'a, T> View<'a, T> {
    /// Returns the number of dimensions of the view.
    pub fn dimensions(&self) -> usize {
        self.shape.len()
    }

    /// Returns an iterator over the elements in the view in row-major order.
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

    /// Returns an owned array corresponding to the view.
    pub fn to_array(&self) -> Array<T>
    where
        T: Clone,
    {
        let data = Vec::from_iter(self.iter().cloned());
        let shape = Shape(self.shape.iter().copied().collect());

        Array::new_unchecked(data, shape)
    }
}
