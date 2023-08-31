//! N-dimensional array.

use std::{
    fmt, io,
    ops::{Index, IndexMut},
};

pub mod iter;
use iter::{AxisIter, IndicesIter};

pub mod npy;

pub(crate) mod shape;
use shape::Strides;
pub use shape::{Axis, Shape};

pub mod view;
use view::View;

/// An N-dimensional strided array.
#[derive(Clone, Debug, PartialEq)]
pub struct Array<T> {
    data: Vec<T>,
    shape: Shape,
    strides: Strides,
}

impl<T> Array<T> {
    /// Returns a mutable reference to the underlying data as a flat slice in row-major order.
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        self.data.as_mut_slice()
    }

    /// Returns the underlying data as a flat slice in row-major order.
    pub fn as_slice(&self) -> &[T] {
        self.data.as_slice()
    }

    /// Returns the number of dimensions of the array.
    pub fn dimensions(&self) -> usize {
        self.shape.len()
    }

    /// Returns the number of elements in the array.
    pub fn elements(&self) -> usize {
        self.data.len()
    }

    /// Creates a new array by repeating a single element to a shape.
    pub fn from_element<S>(element: T, shape: S) -> Self
    where
        T: Clone,
        S: Into<Shape>,
    {
        let shape = shape.into();
        let elements = shape.elements();

        Self::new_unchecked(vec![element; elements], shape)
    }

    /// Creates a new array from an iterator an its shape.
    ///
    /// # Errors
    ///
    /// If the number of items in the iterator does not match the provided shape.
    pub fn from_iter<I, S>(iter: I, shape: S) -> Result<Self, ShapeError>
    where
        I: IntoIterator<Item = T>,
        S: Into<Shape>,
    {
        Self::new(Vec::from_iter(iter), shape)
    }

    /// Returns the element at the provided index if in bounds, and `None` otherwise,
    pub fn get<I>(&self, index: I) -> Option<&T>
    where
        I: AsRef<[usize]>,
    {
        let index = index.as_ref();

        if index.len() == self.dimensions() {
            self.strides
                .flat_index(&self.shape, index)
                .and_then(|flat| self.data.get(flat))
        } else {
            None
        }
    }

    /// Returns a view of the array along the provided axis at the provided index if in bounds, and
    /// `None` otherwise.
    ///
    /// See [`Array::index_axis`] for a panicking version.
    pub fn get_axis(&self, axis: Axis, index: usize) -> Option<View<'_, T>> {
        if axis.0 > self.dimensions() || index >= self.shape[axis.0] {
            None
        } else {
            let offset = index * self.strides[axis.0];
            let data = &self.data[offset..];
            let shape = self.shape.remove_axis(axis);
            let strides = self.strides.remove_axis(axis);

            Some(View::new_unchecked(data, shape, strides))
        }
    }

    /// Returns a mutable reference to the element at the provided index if in bounds, and `None`
    /// otherwise,
    pub fn get_mut<I>(&mut self, index: I) -> Option<&mut T>
    where
        I: AsRef<[usize]>,
    {
        let index = index.as_ref();

        if index.len() == self.dimensions() {
            self.strides
                .flat_index(&self.shape, index)
                .and_then(|flat| self.data.get_mut(flat))
        } else {
            None
        }
    }

    /// Returns a view of the array along the provided axis at the provided index if in bounds.
    ///
    /// # Panics
    ///
    /// If the axis or the index is not in bounds, see [`Array::get_axis`] for a fallible version.
    pub fn index_axis(&self, axis: Axis, index: usize) -> View<'_, T> {
        self.get_axis(axis, index)
            .expect("axis or index out of bounds")
    }

    /// Returns an iterator over the underlying data in row-major order.
    pub fn iter(&self) -> std::slice::Iter<'_, T> {
        self.data.iter()
    }

    /// Returns an iterator over views of the array along the provided axis.
    pub fn iter_axis(&self, axis: Axis) -> AxisIter<'_, T> {
        AxisIter::new(self, axis)
    }

    /// Returns an iterator over indices of the array in row-major order.
    pub fn iter_indices(&self) -> IndicesIter<'_> {
        IndicesIter::new(self)
    }

    /// Returns an iterator over mutable references to the underlying data in row-major order.
    pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, T> {
        self.data.iter_mut()
    }

    /// Creates a new array from data in row-major order and a shape.
    ///
    /// # Errors
    ///
    /// If the number of items in the data does not match the provided shape.
    pub fn new<D, S>(data: D, shape: S) -> Result<Self, ShapeError>
    where
        D: Into<Vec<T>>,
        S: Into<Shape>,
    {
        let data = data.into();
        let shape = shape.into();

        if data.len() == shape.elements() {
            Ok(Array::new_unchecked(data, shape))
        } else {
            Err(ShapeError {
                shape,
                n: data.len(),
            })
        }
    }

    /// Creates a new array from data in row-major order and a shape.
    ///
    /// Prefer using [`Array::new`] to ensure the data fits the provided shape.
    /// It is a logic error where this is not true, though it can not trigger unsafe behaviour.
    pub fn new_unchecked<D, S>(data: D, shape: S) -> Self
    where
        D: Into<Vec<T>>,
        S: Into<Shape>,
    {
        let data = data.into();
        let shape = shape.into();

        Self {
            data,
            strides: shape.strides(),
            shape,
        }
    }

    /// Returns the shape of the array.
    pub fn shape(&self) -> &Shape {
        &self.shape
    }
}

impl Array<f64> {
    /// Creates a new array filled with zeros to a shape.
    pub fn from_zeros<S>(shape: S) -> Self
    where
        S: Into<Shape>,
    {
        Self::from_element(0.0, shape)
    }

    /// Reads an array from the [`npy`] format.
    ///
    /// See the [format docs](https://numpy.org/devdocs/reference/generated/numpy.lib.format.html)
    /// for details.
    pub fn read_npy<R>(mut reader: R) -> io::Result<Self>
    where
        R: io::BufRead,
    {
        npy::read_array(&mut reader)
    }

    /// Returns the sum of the elements in the array.
    pub fn sum(&self, axis: Axis) -> Self {
        let smaller_shape = self.shape.remove_axis(axis).into_shape();

        self.iter_axis(axis)
            .fold(Array::from_zeros(smaller_shape), |mut array, view| {
                array.iter_mut().zip(view.iter()).for_each(|(x, y)| *x += y);
                array
            })
    }

    /// Writes the in the [`npy`] format.
    ///
    /// See the [format docs](https://numpy.org/devdocs/reference/generated/numpy.lib.format.html)
    /// for details.
    pub fn write_npy<W>(&self, mut writer: W) -> io::Result<()>
    where
        W: io::Write,
    {
        npy::write_array(&mut writer, self)
    }
}

impl<T, I> Index<I> for Array<T>
where
    I: AsRef<[usize]>,
{
    type Output = T;

    fn index(&self, index: I) -> &Self::Output {
        self.get(index)
            .expect("index invalid dimension or out of bounds")
    }
}

impl<T, I> IndexMut<I> for Array<T>
where
    I: AsRef<[usize]>,
{
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        self.get_mut(index)
            .expect("index invalid dimension or out of bounds")
    }
}

/// An error associated with a shape mismatch on construction of an [`Array`].
#[derive(Debug)]
pub struct ShapeError {
    shape: Shape,
    n: usize,
}

impl fmt::Display for ShapeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ShapeError { shape, n } = self;
        write!(
            f,
            "cannot construct array with shape {shape} from {n} elements"
        )
    }
}

impl std::error::Error for ShapeError {}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::approx::ApproxEq;

    impl<T> ApproxEq for Array<T>
    where
        T: ApproxEq,
    {
        const DEFAULT_EPSILON: Self::Epsilon = T::DEFAULT_EPSILON;

        type Epsilon = T::Epsilon;

        fn approx_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
            self.data.approx_eq(&other.data, epsilon)
                && self.shape == other.shape
                && self.strides == other.strides
        }
    }
}
