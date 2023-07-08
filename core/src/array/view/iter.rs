use std::iter::FusedIterator;

use super::View;

#[derive(Clone, Debug)]
pub struct Iter<'a, T> {
    view: View<'a, T>,
    index: usize,
    coords: Vec<usize>,
    first: bool,
}

impl<'a, T> Iter<'a, T> {
    pub(super) fn new(view: View<'a, T>) -> Self {
        Self {
            coords: vec![0; view.shape.len()],
            view,
            index: 0,
            first: true,
        }
    }

    fn impl_next(&mut self, dim: usize) -> Option<<Self as Iterator>::Item> {
        if self.first {
            self.first = false;
            return Some(&self.view.data[self.view.offset]);
        }

        self.coords[dim] += 1;

        if self.coords[dim] < self.view.shape[dim] {
            self.index += self.view.strides[dim];

            Some(&self.view.data[self.view.offset..][self.index])
        } else if dim > 0 {
            self.coords[dim] = 0;
            let backstride = self.view.strides[dim] * (self.view.shape[dim] - 1);
            self.index -= backstride;

            self.impl_next(dim - 1)
        } else {
            None
        }
    }
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        self.impl_next(self.view.shape.len() - 1)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = self.view.shape.iter().product();
        (n, Some(n))
    }
}

impl<'a, T> ExactSizeIterator for Iter<'a, T> {}

impl<'a, T> FusedIterator for Iter<'a, T> {}

#[cfg(test)]
mod tests {
    use crate::{array::Axis, Array};

    #[test]
    fn test_iter_fused() {
        let array = Array::new([0.0, 1.0], [2, 1]).unwrap();
        let view = array.get_axis(Axis(0), 0).unwrap();
        let mut iter = view.iter();

        assert_eq!(iter.next(), Some(&0.0));
        assert_eq!(iter.next(), None);
        assert_eq!(iter.next(), None);
    }
}
