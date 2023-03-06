use std::iter::FusedIterator;

use super::View;

#[derive(Clone, Debug)]
pub struct Iter<'a> {
    view: View<'a>,
    index: usize,
    coords: Vec<usize>,
    first: bool,
}

impl<'a> Iter<'a> {
    pub(super) fn new(view: View<'a>) -> Self {
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

impl<'a> Iterator for Iter<'a> {
    type Item = &'a f64;

    fn next(&mut self) -> Option<Self::Item> {
        self.impl_next(self.view.shape.len() - 1)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = self.view.shape.iter().product();
        (n, Some(n))
    }
}

impl<'a> ExactSizeIterator for Iter<'a> {}

impl<'a> FusedIterator for Iter<'a> {}

#[cfg(test)]
mod tests {
    use crate::{Axis, Sfs, Shape};

    #[test]
    fn test_iter_fused() {
        let array = Sfs::new(vec![0.0, 1.0], Shape(vec![2, 1])).unwrap();
        let view = array.get_axis(Axis(0), 0).unwrap();
        let mut iter = view.iter();

        assert_eq!(iter.next(), Some(&0.0));
        assert_eq!(iter.next(), None);
        assert_eq!(iter.next(), None);
    }

    //     #[test]
    //     fn full_3x2() {
    //         let array = Array::from_iter_shape(0..6, Shape::from(vec![3, 2])).unwrap();
    //         let view = array.view();
    //         let values: Vec<i32> = view.iter().copied().collect();
    //         assert_eq!(values, Vec::from_iter(0..6));
    //     }

    //     fn test_index_axis_iter(shape: &[usize], axis: Axis, index: usize, expected: &[i32]) {
    //         let n = shape.iter().product::<usize>();
    //         let array = Array::from_iter_shape(0..(n as i32), Shape::from(shape.to_vec())).unwrap();
    //         let view = array.index_axis(axis, index);
    //         let values: Vec<i32> = view.iter().copied().collect();
    //         assert_eq!(values, expected);
    //     }

    //     #[test]
    //     fn index_axis_3x3() {
    //         let run = |axis, index, expected| test_index_axis_iter(&[3, 3], axis, index, expected);

    //         run(Axis(0), 0, &[0, 1, 2]);
    //         run(Axis(0), 1, &[3, 4, 5]);
    //         run(Axis(0), 2, &[6, 7, 8]);

    //         run(Axis(1), 0, &[0, 3, 6]);
    //         run(Axis(1), 1, &[1, 4, 7]);
    //         run(Axis(1), 2, &[2, 5, 8]);
    //     }

    //     #[test]
    //     fn index_axis_3x2x4() {
    //         let run = |axis, index, expected| test_index_axis_iter(&[3, 2, 4], axis, index, expected);

    //         run(Axis(0), 0, &[0, 1, 2, 3, 4, 5, 6, 7]);
    //         run(Axis(0), 1, &[8, 9, 10, 11, 12, 13, 14, 15]);
    //         run(Axis(0), 2, &[16, 17, 18, 19, 20, 21, 22, 23]);

    //         run(Axis(1), 0, &[0, 1, 2, 3, 8, 9, 10, 11, 16, 17, 18, 19]);
    //         run(Axis(1), 1, &[4, 5, 6, 7, 12, 13, 14, 15, 20, 21, 22, 23]);

    //         run(Axis(2), 0, &[0, 4, 8, 12, 16, 20]);
    //         run(Axis(2), 1, &[1, 5, 9, 13, 17, 21]);
    //         run(Axis(2), 2, &[2, 6, 10, 14, 18, 22]);
    //         run(Axis(2), 3, &[3, 7, 11, 15, 19, 23]);
    //     }

    //     #[test]
    //     fn index_axis_2x1x3x2() {
    //         let run =
    //             |axis, index, expected| test_index_axis_iter(&[2, 1, 3, 2], axis, index, expected);

    //         run(Axis(0), 0, &[0, 1, 2, 3, 4, 5]);
    //         run(Axis(0), 1, &[6, 7, 8, 9, 10, 11]);

    //         run(Axis(1), 0, &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11]);

    //         run(Axis(2), 0, &[0, 1, 6, 7]);
    //         run(Axis(2), 1, &[2, 3, 8, 9]);
    //         run(Axis(2), 2, &[4, 5, 10, 11]);
    //     }
}
