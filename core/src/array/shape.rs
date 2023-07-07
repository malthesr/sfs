use std::{fmt, ops::Deref};

mod removed_axis;
pub(crate) use removed_axis::RemovedAxis;

mod strides;
pub use strides::Strides;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Axis(pub usize);

impl Deref for Axis {
    type Target = usize;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Shape(pub Vec<usize>);

impl Shape {
    pub fn elements(&self) -> usize {
        self.iter().product()
    }

    pub(crate) fn index_from_flat_unchecked(&self, mut flat: usize) -> Vec<usize> {
        let mut n = self.elements();
        let mut index = vec![0; self.len()];
        for (i, v) in self.iter().enumerate() {
            n /= v;
            index[i] = flat / n;
            flat %= n;
        }
        index
    }

    pub(crate) fn index_sum_from_flat_unchecked(&self, mut flat: usize) -> usize {
        let mut n = self.elements();
        let mut sum = 0;
        for v in self.iter() {
            n /= v;
            sum += flat / n;
            flat %= n;
        }
        sum
    }

    pub(crate) fn remove_axis(&self, axis: Axis) -> RemovedAxis<Self> {
        RemovedAxis::new(self, axis)
    }

    pub(crate) fn strides(&self) -> Strides {
        let mut strides = vec![1; self.len()];

        for (i, v) in self.iter().enumerate().skip(1).rev() {
            strides.iter_mut().take(i).for_each(|stride| *stride *= v)
        }

        Strides(strides)
    }
}

impl AsRef<[usize]> for Shape {
    fn as_ref(&self) -> &[usize] {
        self
    }
}

impl Deref for Shape {
    type Target = [usize];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<Vec<usize>> for Shape {
    fn from(shape: Vec<usize>) -> Self {
        Self(shape)
    }
}

impl fmt::Display for Shape {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self[0])?;
        for v in self.iter().skip(1) {
            write!(f, "/{v}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_index_from_flat_unchecked() {
        let shape = Shape(vec![3, 3, 4]);

        assert_eq!(shape.index_from_flat_unchecked(0), vec![0, 0, 0]);
        assert_eq!(shape.index_from_flat_unchecked(1), vec![0, 0, 1]);
        assert_eq!(shape.index_from_flat_unchecked(3), vec![0, 0, 3]);
        assert_eq!(shape.index_from_flat_unchecked(4), vec![0, 1, 0]);
        assert_eq!(shape.index_from_flat_unchecked(35), vec![2, 2, 3]);
    }

    #[test]
    fn test_strides() {
        let shape = Shape(vec![6, 3, 7]);
        let strides = shape.strides();

        assert_eq!(strides, Strides(vec![21, 7, 1]));
    }
}
