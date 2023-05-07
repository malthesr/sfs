use std::ops::Index;

use super::Axis;

#[derive(Debug, Eq, Hash, PartialEq)]
pub struct RemovedAxis<'a, T> {
    inner: &'a T,
    removed: Axis,
}

impl<'a, T> RemovedAxis<'a, T>
where
    T: AsRef<[usize]>,
{
    pub fn get(&self, index: usize) -> Option<&'a usize> {
        let inner = self.inner.as_ref();

        if index < *self.removed {
            inner.get(index)
        } else {
            inner.get(index + 1)
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &'a usize> {
        let inner = self.inner.as_ref();

        inner[..*self.removed]
            .iter()
            .chain(&inner[1 + *self.removed..])
    }

    pub fn len(&self) -> usize {
        self.inner.as_ref().len() - 1
    }

    pub fn new(inner: &'a T, removed: Axis) -> Self {
        if !inner.as_ref().is_empty() {
            Self { inner, removed }
        } else {
            panic!("cannot remove axis from empty")
        }
    }
}

impl<'a, T> Clone for RemovedAxis<'a, T>
where
    &'a T: Copy,
{
    fn clone(&self) -> Self {
        *self
    }
}

impl<'a, T> Copy for RemovedAxis<'a, T> where &'a T: Copy {}

impl<'a, T> Index<usize> for RemovedAxis<'a, T>
where
    T: AsRef<[usize]>,
{
    type Output = usize;

    fn index(&self, index: usize) -> &Self::Output {
        self.get(index).expect("index out of bounds")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::Shape;

    #[test]
    #[should_panic]
    fn test_removed_axis_empty() {
        _ = RemovedAxis::new(&[], Axis(0));
    }

    #[test]
    fn test_removed_axis_get() {
        let shape = Shape(vec![0, 1, 2, 3, 4]);
        let removed_axis = RemovedAxis::new(&shape, Axis(2));

        assert_eq!(removed_axis.get(0), Some(&0));
        assert_eq!(removed_axis.get(1), Some(&1));
        assert_eq!(removed_axis.get(2), Some(&3));
        assert_eq!(removed_axis.get(3), Some(&4));
        assert_eq!(removed_axis.get(4), None);
    }

    #[test]
    fn test_removed_axis_iter() {
        let shape = Shape(vec![0, 1, 2, 3, 4]);
        let removed_axis = RemovedAxis::new(&shape, Axis(0));
        let mut iter = removed_axis.iter();

        assert_eq!(iter.next(), Some(&1));
        assert_eq!(iter.next(), Some(&2));
        assert_eq!(iter.next(), Some(&3));
        assert_eq!(iter.next(), Some(&4));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn test_removed_axis_len() {
        let shape = Shape(vec![0, 1]);
        let removed_axis = RemovedAxis::new(&shape, Axis(0));

        assert_eq!(removed_axis.len(), 1);
    }
}
