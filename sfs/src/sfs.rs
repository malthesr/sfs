use std::ops::{Index, IndexMut};

use crate::{shape::Strides, Axis, Shape, View};

mod iter;
pub use iter::AxisIter;

#[derive(Clone, Debug, PartialEq)]
pub struct Sfs {
    pub(crate) data: Vec<f64>,
    pub(crate) shape: Shape,
    pub(crate) strides: Strides,
}

impl Sfs {
    pub fn as_slice(&self) -> &[f64] {
        self.data.as_slice()
    }

    pub fn dimensions(&self) -> usize {
        self.shape.len()
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

    pub fn iter(&self) -> std::slice::Iter<'_, f64> {
        self.data.iter()
    }

    pub fn iter_axis(&self, axis: Axis) -> AxisIter<'_> {
        AxisIter::new(self, axis)
    }

    pub fn new(data: Vec<f64>, shape: Shape) -> Option<Self> {
        if data.len() == shape.iter().product() {
            Some(Self::new_unchecked(data, shape))
        } else {
            None
        }
    }

    pub fn new_unchecked(data: Vec<f64>, shape: Shape) -> Self {
        Self {
            data,
            strides: shape.strides(),
            shape,
        }
    }

    pub fn shape(&self) -> &Shape {
        &self.shape
    }

    pub fn zeros(shape: Shape) -> Self {
        Self::new_unchecked(vec![0.0; shape.iter().product()], shape)
    }
}

impl<I> Index<I> for Sfs
where
    I: AsRef<[usize]>,
{
    type Output = f64;

    fn index(&self, index: I) -> &Self::Output {
        self.strides
            .flat_index(&self.shape, index)
            .and_then(|flat| self.data.get(flat))
            .expect("index out of bounds")
    }
}

impl<I> IndexMut<I> for Sfs
where
    I: AsRef<[usize]>,
{
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        self.strides
            .flat_index(&self.shape, index)
            .and_then(|flat| self.data.get_mut(flat))
            .expect("index out of bounds")
    }
}
