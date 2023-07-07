use std::iter::FusedIterator;

use crate::array::iter::IndicesIter;

use super::Sfs;

#[derive(Debug)]
pub struct FrequenciesIter<'a> {
    inner: IndicesIter<'a>,
}

impl<'a> FrequenciesIter<'a> {
    pub(super) fn new<const N: bool>(sfs: &'a Sfs<N>) -> Self {
        Self {
            inner: sfs.array.iter_indices(),
        }
    }
}

impl<'a> Iterator for FrequenciesIter<'a> {
    type Item = Vec<f64>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|indices| {
            indices
                .iter()
                .zip(self.inner.array().shape().iter())
                .map(|(&i, n)| i as f64 / (n - 1) as f64)
                .collect()
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<'a> ExactSizeIterator for FrequenciesIter<'a> {}

impl<'a> FusedIterator for FrequenciesIter<'a> {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_iter_frequencies_2d() {
        let sfs = Sfs::from_zeros(vec![2, 3]);
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
