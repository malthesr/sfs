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
pub struct Array {
    data: Vec<f64>,
    shape: Shape,
    strides: Strides,
}

impl Array {
    pub fn as_mut_slice(&mut self) -> &mut [f64] {
        self.data.as_mut_slice()
    }

    pub fn as_slice(&self) -> &[f64] {
        self.data.as_slice()
    }

    pub fn dimensions(&self) -> usize {
        self.shape.len()
    }

    pub fn elements(&self) -> usize {
        self.data.len()
    }

    pub fn from_iter<I, S>(iter: I, shape: S) -> Result<Self, ShapeError>
    where
        I: IntoIterator<Item = f64>,
        Shape: From<S>,
    {
        let data = iter.into_iter().collect();

        Self::new(data, shape)
    }

    pub fn from_zeros<S>(shape: S) -> Self
    where
        Shape: From<S>,
    {
        let shape = Shape::from(shape);
        let elements = shape.elements();

        Self::new_unchecked::<Shape>(vec![0.0; elements], shape)
    }

    pub fn get<I>(&self, index: I) -> Option<&f64>
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

    pub fn get_axis(&self, axis: Axis, index: usize) -> Option<View<'_>> {
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

    pub fn get_mut<I>(&mut self, index: I) -> Option<&mut f64>
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

    pub fn iter(&self) -> std::slice::Iter<'_, f64> {
        self.data.iter()
    }

    pub fn iter_axis(&self, axis: Axis) -> AxisIter<'_> {
        AxisIter::new(self, axis)
    }

    pub fn iter_indices(&self) -> IndicesIter<'_> {
        IndicesIter::new(self)
    }

    pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, f64> {
        self.data.iter_mut()
    }

    pub fn new<S>(data: Vec<f64>, shape: S) -> Result<Self, ShapeError>
    where
        Shape: From<S>,
    {
        let shape = Shape::from(shape);

        if data.len() == shape.elements() {
            Ok(Self::new_unchecked::<Shape>(data, shape))
        } else {
            Err(ShapeError {
                shape,
                n: data.len(),
            })
        }
    }

    pub fn new_unchecked<S>(data: Vec<f64>, shape: S) -> Self
    where
        Shape: From<S>,
    {
        let shape = Shape::from(shape);

        Self {
            data,
            strides: shape.strides(),
            shape,
        }
    }

    pub fn read_npy<R>(mut reader: R) -> io::Result<Self>
    where
        R: io::BufRead,
    {
        npy::read_array(&mut reader)
    }

    pub fn shape(&self) -> &Shape {
        &self.shape
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

impl<I> Index<I> for Array
where
    I: AsRef<[usize]>,
{
    type Output = f64;

    fn index(&self, index: I) -> &Self::Output {
        self.get(index)
            .expect("index invalid dimension or out of bounds")
    }
}

impl<I> IndexMut<I> for Array
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
