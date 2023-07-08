use std::iter::FusedIterator;

use crate::array::iter::IndicesIter;

use super::{Spectrum, State};

#[derive(Debug)]
pub struct FrequenciesIter<'a> {
    inner: IndicesIter<'a>,
}

impl<'a> FrequenciesIter<'a> {
    pub(super) fn new<S: State>(spectrum: &'a Spectrum<S>) -> Self {
        Self {
            inner: spectrum.array.iter_indices(),
        }
    }
}

impl<'a> Iterator for FrequenciesIter<'a> {
    type Item = Vec<f64>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|indices| {
            indices
                .iter()
                .zip(self.inner.shape().iter())
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
        let spectrum = Spectrum::from_zeros([2, 3]);
        let mut iter = spectrum.iter_frequencies();

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
