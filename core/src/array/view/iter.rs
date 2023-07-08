use std::iter::FusedIterator;

use super::View;

#[derive(Clone, Debug)]
pub struct Iter<'a, T> {
    view: View<'a, T>,
    index: usize,
    coords: Option<Vec<usize>>,
}

impl<'a, T> Iter<'a, T> {
    pub(super) fn new(view: View<'a, T>) -> Self {
        Self {
            view,
            index: 0,
            coords: None,
        }
    }

    fn backstride(&self, axis: usize) -> usize {
        self.view.strides[axis] * (self.view.shape[axis] - 1)
    }

    fn impl_next_rec(&mut self, axis: usize) -> Option<<Self as Iterator>::Item> {
        let coords = if let Some(coords) = self.coords.as_mut() {
            coords
        } else {
            self.coords = Some(vec![0; self.view.dimensions()]);
            return self.view.data.first();
        };

        coords[axis] += 1;

        if coords[axis] < self.view.shape[axis] {
            self.index += self.view.strides[axis];

            Some(&self.view.data[self.index])
        } else if axis > 0 {
            coords[axis] = 0;
            self.index -= self.backstride(axis);

            self.impl_next_rec(axis - 1)
        } else {
            None
        }
    }
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        self.impl_next_rec(self.view.dimensions() - 1)
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

    macro_rules! assert_iter_eq {
        ($array:ident [axis: $axis:literal, index: $index:literal], [$($expected:literal),* $(,)?] $(,)?) => {
            assert_eq!(
                Vec::from_iter($array.index_axis(Axis($axis), $index).iter().copied()),
                vec![$($expected),+],
            );
        };
    }

    #[test]
    fn test_iter_2x2() {
        let array = Array::from_iter(0..4, [2, 2]).unwrap();

        assert_iter_eq!(array[axis: 0, index: 0], [0, 1]);
        assert_iter_eq!(array[axis: 0, index: 1], [2, 3]);

        assert_iter_eq!(array[axis: 1, index: 0], [0, 2]);
        assert_iter_eq!(array[axis: 1, index: 1], [1, 3]);
    }

    #[test]
    fn test_iter_2x3x2() {
        let array = Array::from_iter(0..12, [2, 3, 2]).unwrap();

        assert_iter_eq!(array[axis: 0, index: 0], [0, 1, 2, 3, 4, 5]);
        assert_iter_eq!(array[axis: 0, index: 1], [6, 7, 8, 9, 10, 11]);

        assert_iter_eq!(array[axis: 1, index: 0], [0, 1, 6, 7]);
        assert_iter_eq!(array[axis: 1, index: 1], [2, 3, 8, 9]);
        assert_iter_eq!(array[axis: 1, index: 2], [4, 5, 10, 11]);

        assert_iter_eq!(array[axis: 2, index: 0], [0, 2, 4, 6, 8, 10]);
        assert_iter_eq!(array[axis: 2, index: 1], [1, 3, 5, 7, 9, 11]);
    }

    #[test]
    fn test_iter_2x1x2x3() {
        let array = Array::from_iter(0..12, [2, 1, 2, 3]).unwrap();

        assert_iter_eq!(array[axis: 0, index: 0], [0, 1, 2, 3, 4, 5]);
        assert_iter_eq!(array[axis: 0, index: 1], [6, 7, 8, 9, 10, 11]);

        assert_iter_eq!(array[axis: 1, index: 0], [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11]);

        assert_iter_eq!(array[axis: 2, index: 0], [0, 1, 2, 6, 7, 8]);
        assert_iter_eq!(array[axis: 2, index: 1], [3, 4, 5, 9, 10, 11]);

        assert_iter_eq!(array[axis: 3, index: 0], [0, 3, 6, 9]);
        assert_iter_eq!(array[axis: 3, index: 1], [1, 4, 7, 10]);
        assert_iter_eq!(array[axis: 3, index: 2], [2, 5, 8, 11]);
    }

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
