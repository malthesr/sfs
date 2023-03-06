use std::ops::Deref;

mod removed_axis;
pub use removed_axis::RemovedAxis;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strides() {
        let shape = Shape(vec![6, 3, 7]);
        let strides = shape.strides();

        assert_eq!(strides, Strides(vec![21, 7, 1]));
    }
}
