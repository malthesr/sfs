use std::iter::FusedIterator;

use super::{Array, Axis, View};

#[derive(Debug)]
pub struct AxisIter<'a> {
    array: &'a Array,
    axis: Axis,
    index: usize,
}

impl<'a> AxisIter<'a> {
    pub(super) fn new(array: &'a Array, axis: Axis) -> Self {
        Self {
            array,
            axis,
            index: 0,
        }
    }
}

impl<'a> Iterator for AxisIter<'a> {
    type Item = View<'a>;

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

impl<'a> ExactSizeIterator for AxisIter<'a> {}

impl<'a> FusedIterator for AxisIter<'a> {}

#[derive(Debug)]
pub struct IndicesIter<'a> {
    array: &'a Array,
    index: usize,
    total: usize,
}

impl<'a> IndicesIter<'a> {
    pub(crate) fn array(&self) -> &'a Array {
        self.array
    }

    pub(super) fn new(array: &'a Array) -> Self {
        Self {
            array,
            index: 0,
            total: array.shape.iter().product::<usize>(),
        }
    }
}

impl<'a> Iterator for IndicesIter<'a> {
    type Item = Vec<usize>;

    fn next(&mut self) -> Option<Self::Item> {
        (self.index < self.total).then(|| {
            self.index += 1;
            self.array.shape.index_from_flat_unchecked(self.index - 1)
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

    use crate::array::Shape;

    #[test]
    fn test_iter_indices_1d() {
        let sfs = Array::from_zeros(Shape(vec![4]));
        let mut iter = sfs.iter_indices();

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
        let sfs = Array::from_zeros(Shape(vec![2, 3]));
        let mut iter = sfs.iter_indices();

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
        let sfs = Array::from_zeros(Shape(vec![2, 1, 3]));
        let mut iter = sfs.iter_indices();

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
