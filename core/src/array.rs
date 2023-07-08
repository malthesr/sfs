pub mod iter;
use std::{
    fmt, io,
    ops::{Index, IndexMut},
};

use iter::{AxisIter, IndicesIter};

pub mod npy;

pub mod shape;
use shape::Strides;
pub use shape::{Axis, Shape};

pub mod view;
use view::View;

#[derive(Clone, Debug, PartialEq)]
pub struct Array<T> {
    data: Vec<T>,
    shape: Shape,
    strides: Strides,
}

impl<T> Array<T> {
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        self.data.as_mut_slice()
    }

    pub fn as_slice(&self) -> &[T] {
        self.data.as_slice()
    }

    pub fn dimensions(&self) -> usize {
        self.shape.len()
    }

    pub fn elements(&self) -> usize {
        self.data.len()
    }

    pub fn from_element<S>(element: T, shape: S) -> Self
    where
        T: Clone,
        Shape: From<S>,
    {
        let shape = Shape::from(shape);
        let elements = shape.elements();

        Self::new_unchecked::<_, Shape>(vec![element; elements], shape)
    }

    pub fn from_iter<I, S>(iter: I, shape: S) -> Result<Self, ShapeError>
    where
        I: IntoIterator<Item = T>,
        Shape: From<S>,
    {
        Self::new(Vec::from_iter(iter), shape)
    }

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

    pub fn get_axis(&self, axis: Axis, index: usize) -> Option<View<'_, T>> {
        if axis.0 > self.dimensions() || index >= self.shape[axis.0] {
            None
        } else {
            let offset = index * self.strides[axis.0];
            let data = &self.data;
            let shape = self.shape.remove_axis(axis);
            let strides = self.strides.remove_axis(axis);

            Some(View::new_unchecked(data, offset, shape, strides))
        }
    }

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

    pub fn index_axis(&self, axis: Axis, index: usize) -> View<'_, T> {
        self.get_axis(axis, index)
            .expect("axis or index out of bounds")
    }

    pub fn iter(&self) -> std::slice::Iter<'_, T> {
        self.data.iter()
    }

    pub fn iter_axis(&self, axis: Axis) -> AxisIter<'_, T> {
        AxisIter::new(self, axis)
    }

    pub fn iter_indices(&self) -> IndicesIter<'_> {
        IndicesIter::new(self)
    }

    pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, T> {
        self.data.iter_mut()
    }

    pub fn new<D, S>(data: D, shape: S) -> Result<Self, ShapeError>
    where
        Vec<T>: From<D>,
        Shape: From<S>,
    {
        let data = Vec::from(data);
        let shape = Shape::from(shape);

        if data.len() == shape.elements() {
            Ok(Array::new_unchecked::<Vec<T>, Shape>(data, shape))
        } else {
            Err(ShapeError {
                shape,
                n: data.len(),
            })
        }
    }

    pub fn new_unchecked<D, S>(data: D, shape: S) -> Self
    where
        Vec<T>: From<D>,
        Shape: From<S>,
    {
        let data = Vec::from(data);
        let shape = Shape::from(shape);

        Self {
            data,
            strides: shape.strides(),
            shape,
        }
    }

    pub fn shape(&self) -> &Shape {
        &self.shape
    }
}

impl Array<f64> {
    pub fn from_zeros<S>(shape: S) -> Self
    where
        Shape: From<S>,
    {
        Self::from_element(0.0, shape)
    }

    pub fn read_npy<R>(mut reader: R) -> io::Result<Self>
    where
        R: io::BufRead,
    {
        npy::read_array(&mut reader)
    }

    pub fn sum(&self, axis: Axis) -> Self {
        let smaller_shape = self.shape.remove_axis(axis).into_shape();

        self.iter_axis(axis)
            .fold(Array::from_zeros(smaller_shape), |mut array, view| {
                array.iter_mut().zip(view.iter()).for_each(|(x, y)| *x += y);
                array
            })
    }

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
