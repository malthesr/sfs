use std::ops::Deref;

use crate::Shape;

use super::{Axis, RemovedAxis};

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Strides(pub Vec<usize>);

impl Strides {
    pub(crate) fn flat_index<I>(&self, shape: &Shape, index: I) -> Option<usize>
    where
        I: AsRef<[usize]>,
    {
        let index = index.as_ref();

        let dimensions_match = self.len() == shape.len() && shape.len() == index.len();

        if dimensions_match {
            let in_bounds = index
                .iter()
                .zip(shape.iter())
                .all(|(idx, shape)| idx < shape);

            if in_bounds {
                Some(self.flat_index_unchecked(index))
            } else {
                None
            }
        } else {
            None
        }
    }

    pub(crate) fn flat_index_unchecked<I>(&self, index: I) -> usize
    where
        I: AsRef<[usize]>,
    {
        self.iter()
            .zip(index.as_ref())
            .fold(0, |flat, (stride, idx)| flat + stride * idx)
    }

    pub(crate) fn remove_axis(&self, axis: Axis) -> RemovedAxis<Self> {
        RemovedAxis::new(self, axis)
    }
}

impl AsRef<[usize]> for Strides {
    fn as_ref(&self) -> &[usize] {
        self
    }
}

impl Deref for Strides {
    type Target = [usize];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flat_index() {
        let shape = Shape(vec![5, 4, 9, 2]);
        let strides = shape.strides();

        assert_eq!(strides.flat_index(&shape, &[0, 0, 0, 0]), Some(0));
        assert_eq!(strides.flat_index(&shape, &[0, 0, 0, 1]), Some(1));
        assert_eq!(strides.flat_index(&shape, &[0, 0, 1, 0]), Some(2));
        assert_eq!(strides.flat_index(&shape, &[0, 1, 0, 0]), Some(18));
        assert_eq!(strides.flat_index(&shape, &[1, 0, 0, 0]), Some(72));
        assert_eq!(strides.flat_index(&shape, &[4, 3, 8, 1]), Some(359));
    }

    #[test]
    fn test_flat_index_dimension_mismatch() {
        let strides = Strides(vec![1]);

        assert_eq!(strides.flat_index(&Shape(vec![1]), &[]), None);
        assert_eq!(strides.flat_index(&Shape(vec![1]), &[0, 0]), None);
        assert_eq!(strides.flat_index(&Shape(vec![]), &[0]), None);
        assert_eq!(strides.flat_index(&Shape(vec![1, 1]), &[0]), None);
        assert_eq!(strides.flat_index(&Shape(vec![1, 1]), &[0, 0]), None);
    }

    #[test]
    fn test_flat_index_out_of_bounds() {
        let shape = Shape(vec![5, 4, 9, 2]);
        let strides = shape.strides();

        assert_eq!(strides.flat_index(&shape, &[5, 3, 8, 1]), None);
        assert_eq!(strides.flat_index(&shape, &[4, 4, 8, 1]), None);
        assert_eq!(strides.flat_index(&shape, &[4, 3, 9, 1]), None);
        assert_eq!(strides.flat_index(&shape, &[4, 3, 8, 2]), None);
    }
}
