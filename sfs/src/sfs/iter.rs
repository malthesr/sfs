use std::iter::FusedIterator;

use crate::{Axis, Sfs, View};

#[derive(Clone, Debug)]
pub struct AxisIter<'a> {
    sfs: &'a Sfs,
    axis: Axis,
    index: usize,
}

impl<'a> AxisIter<'a> {
    pub(super) fn new(sfs: &'a Sfs, axis: Axis) -> Self {
        Self {
            sfs,
            axis,
            index: 0,
        }
    }
}

impl<'a> Iterator for AxisIter<'a> {
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

impl<'a> ExactSizeIterator for AxisIter<'a> {}

impl<'a> FusedIterator for AxisIter<'a> {}
