//! Array iterators.
//!
//! The types in this module are constructed via methods on [`Array`],
//! and generally expose no functionality other than being iterable.

use std::iter::FusedIterator;

use super::{Array, Axis, Shape, View};

/// An iterator over [`View`]s along an axis of an [`Array`].
///
/// See [`Array::iter_axis`] for details.
#[derive(Debug)]
pub struct AxisIter<'a, T> {
    array: &'a Array<T>,
    axis: Axis,
    index: usize,
}

impl<'a, T> AxisIter<'a, T> {
    pub(super) fn new(array: &'a Array<T>, axis: Axis) -> Self {
        Self {
            array,
            axis,
            index: 0,
        }
    }
}

impl<'a, T> Iterator for AxisIter<'a, T> {
    type Item = View<'a, T>;

    fn next(&mut self) -> Option<Self::Item> {
        let view = self.array.get_axis(self.axis, self.index)?;
        self.index += 1;
        Some(view)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = self.array.shape[self.axis.0];
        (n, Some(n))
    }
}

impl<'a, T> ExactSizeIterator for AxisIter<'a, T> {}

impl<'a, T> FusedIterator for AxisIter<'a, T> {}

/// An iterator over indices of elements in an array in row-major order.
///
/// See [`Array::iter_indices`] for details.
#[derive(Debug)]
pub struct IndicesIter<'a> {
    shape: &'a Shape,
    index: usize,
    total: usize,
}

impl<'a> IndicesIter<'a> {
    pub(crate) fn shape(&self) -> &'a Shape {
        self.shape
    }

    pub(crate) fn new<T>(array: &'a Array<T>) -> Self {
        Self::from_shape(array.shape())
    }

    pub(crate) fn from_shape(shape: &'a Shape) -> Self {
        Self {
            shape,
            index: 0,
            total: shape.elements(),
        }
    }
}

impl<'a> Iterator for IndicesIter<'a> {
    type Item = Vec<usize>;

    fn next(&mut self) -> Option<Self::Item> {
        (self.index < self.total).then(|| {
            self.index += 1;
            self.shape.index_from_flat_unchecked(self.index - 1)
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.total - self.index;
        (len, Some(len))
    }
}

impl<'a> ExactSizeIterator for IndicesIter<'a> {}

impl<'a> FusedIterator for IndicesIter<'a> {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_iter_indices_1d() {
        let array = Array::from_zeros(4);
        let mut iter = array.iter_indices();

        assert_eq!(iter.len(), 4);

        assert_eq!(iter.next(), Some(vec![0]));
        assert_eq!(iter.next(), Some(vec![1]));

        assert_eq!(iter.len(), 2);

        assert_eq!(iter.next(), Some(vec![2]));
        assert_eq!(iter.next(), Some(vec![3]));

        assert_eq!(iter.len(), 0);
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_iter_indices_2d() {
        let array = Array::from_zeros([2, 3]);
        let mut iter = array.iter_indices();

        assert_eq!(iter.len(), 6);

        assert_eq!(iter.next(), Some(vec![0, 0]));
        assert_eq!(iter.next(), Some(vec![0, 1]));
        assert_eq!(iter.next(), Some(vec![0, 2]));

        assert_eq!(iter.len(), 3);

        assert_eq!(iter.next(), Some(vec![1, 0]));
        assert_eq!(iter.next(), Some(vec![1, 1]));
        assert_eq!(iter.next(), Some(vec![1, 2]));

        assert_eq!(iter.len(), 0);
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_iter_indices_3d() {
        let array = Array::from_zeros([2, 1, 3]);
        let mut iter = array.iter_indices();

        assert_eq!(iter.next(), Some(vec![0, 0, 0]));
        assert_eq!(iter.next(), Some(vec![0, 0, 1]));
        assert_eq!(iter.next(), Some(vec![0, 0, 2]));

        assert_eq!(iter.len(), 3);

        assert_eq!(iter.next(), Some(vec![1, 0, 0]));
        assert_eq!(iter.next(), Some(vec![1, 0, 1]));
        assert_eq!(iter.next(), Some(vec![1, 0, 2]));

        assert_eq!(iter.len(), 0);
        assert!(iter.next().is_none());
    }
}
