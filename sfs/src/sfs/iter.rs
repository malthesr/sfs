use std::iter::FusedIterator;

use crate::{Axis, Sfs, View};

#[derive(Debug)]
pub struct AxisIter<'a, const N: bool> {
    sfs: &'a Sfs<N>,
    axis: Axis,
    index: usize,
}

impl<'a, const N: bool> AxisIter<'a, N> {
    pub(super) fn new(sfs: &'a Sfs<N>, axis: Axis) -> Self {
        Self {
            sfs,
            axis,
            index: 0,
        }
    }
}

impl<'a, const N: bool> Iterator for AxisIter<'a, N> {
    type Item = View<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let view = self.sfs.get_axis(self.axis, self.index)?;
        self.index += 1;
        Some(view)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = self.sfs.shape[self.axis.0];
        (n, Some(n))
    }
}

impl<'a, const N: bool> ExactSizeIterator for AxisIter<'a, N> {}

impl<'a, const N: bool> FusedIterator for AxisIter<'a, N> {}

#[derive(Debug)]
pub struct IndicesIter<'a, const N: bool> {
    sfs: &'a Sfs<N>,
    index: usize,
    total: usize,
}

impl<'a, const N: bool> IndicesIter<'a, N> {
    pub(super) fn new(sfs: &'a Sfs<N>) -> Self {
        Self {
            sfs,
            index: 0,
            total: sfs.shape.iter().product::<usize>(),
        }
    }
}

impl<'a, const N: bool> Iterator for IndicesIter<'a, N> {
    type Item = Vec<usize>;

    fn next(&mut self) -> Option<Self::Item> {
        (self.index < self.total).then(|| {
            self.index += 1;
            self.sfs.shape.index_from_flat_unchecked(self.index - 1)
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.total - self.index;
        (len, Some(len))
    }
}

impl<'a, const N: bool> ExactSizeIterator for IndicesIter<'a, N> {}

impl<'a, const N: bool> FusedIterator for IndicesIter<'a, N> {}

#[derive(Debug)]
pub struct FrequenciesIter<'a, const N: bool> {
    inner: IndicesIter<'a, N>,
}

impl<'a, const N: bool> FrequenciesIter<'a, N> {
    pub(super) fn new(sfs: &'a Sfs<N>) -> Self {
        Self {
            inner: sfs.iter_indices(),
        }
    }
}

impl<'a, const N: bool> Iterator for FrequenciesIter<'a, N> {
    type Item = Vec<f64>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|indices| {
            indices
                .iter()
                .zip(self.inner.sfs.shape.iter())
                .map(|(&i, n)| i as f64 / (n - 1) as f64)
                .collect()
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<'a, const N: bool> ExactSizeIterator for FrequenciesIter<'a, N> {}

impl<'a, const N: bool> FusedIterator for FrequenciesIter<'a, N> {}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::Shape;

    #[test]
    fn test_iter_indices_1d() {
        let sfs = Sfs::from_zeros(Shape(vec![4]));
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
        let sfs = Sfs::from_zeros(Shape(vec![2, 3]));
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
        let sfs = Sfs::from_zeros(Shape(vec![2, 1, 3]));
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

    #[test]
    fn test_iter_frequencies_2d() {
        let sfs = Sfs::from_zeros(Shape(vec![2, 3]));
        let mut iter = sfs.iter_frequencies();

        assert_eq!(iter.len(), 6);

        assert_eq!(iter.next(), Some(vec![0., 0.]));
        assert_eq!(iter.next(), Some(vec![0., 0.5]));
        assert_eq!(iter.next(), Some(vec![0., 1.]));

        assert_eq!(iter.len(), 3);

        assert_eq!(iter.next(), Some(vec![1., 0.]));
        assert_eq!(iter.next(), Some(vec![1., 0.5]));
        assert_eq!(iter.next(), Some(vec![1., 1.]));

        assert_eq!(iter.len(), 0);
        assert!(iter.next().is_none());
    }
}
