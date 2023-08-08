use crate::array::Shape;

mod hypergeometric;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Projection {
    from: Shape,
    to: Shape,
}

impl Projection {
    pub fn dimensions(&self) -> usize {
        self.from.dimensions()
    }

    pub fn new_unchecked(from: Shape, to: Shape) -> Self {
        Self { from, to }
    }

    fn project_unchecked(&self, from: &[usize], to: &[usize]) -> f64 {
        self.from
            .iter()
            .map(|x| x - 1)
            .zip(from.iter())
            .zip(self.to.iter().map(|x| x - 1))
            .zip(to.iter())
            .map(|(((size, &successes), draws), &observed)| {
                hypergeometric::pmf_unchecked(
                    size as u64,
                    successes as u64,
                    draws as u64,
                    observed as u64,
                )
            })
            .fold(1.0, |joint, probability| joint * probability)
    }

    pub fn project_all_unchecked<'a>(&'a self, from: &'a [usize]) -> ProjectIter<'a> {
        ProjectIter::new(self, from)
    }
}

#[derive(Debug)]
pub struct ProjectIter<'a> {
    projection: &'a Projection,
    from: &'a [usize],
    to: Vec<usize>,
    index: usize,
}

impl<'a> ProjectIter<'a> {
    fn dimensions(&self) -> usize {
        self.to.len()
    }

    fn project_unchecked(&self) -> f64 {
        self.projection.project_unchecked(self.from, &self.to)
    }

    fn impl_next_rec(&mut self, axis: usize) -> Option<<Self as Iterator>::Item> {
        if self.index == 0 {
            self.index += 1;
            return Some(self.project_unchecked());
        };

        self.to[axis] += 1;
        if self.to[axis] < self.projection.to[axis] {
            self.index += 1;
            Some(self.project_unchecked())
        } else if axis > 0 {
            self.to[axis] = 0;
            self.impl_next_rec(axis - 1)
        } else {
            None
        }
    }

    fn new(projection: &'a Projection, from: &'a [usize]) -> Self {
        Self {
            projection,
            from,
            to: vec![0; from.len()],
            index: 0,
        }
    }
}

impl<'a> Iterator for ProjectIter<'a> {
    type Item = f64;

    fn next(&mut self) -> Option<Self::Item> {
        self.impl_next_rec(self.dimensions() - 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_project_7_to_3_project_2() {
        let projection = Projection::new_unchecked(Shape::from(7), Shape::from(3));

        assert_approx_eq!(
            projection.project_all_unchecked(&[2]).collect::<Vec<_>>(),
            vec![0.4, 0.533333, 0.066667],
            epsilon = 1e-6
        );
    }

    #[test]
    fn test_project_3x3_to_2x2() {
        let projection = Projection::new_unchecked(Shape::from([3, 3]), Shape::from([2, 2]));

        macro_rules! assert_project_to {
            ($projection:ident from [$($from:literal),+] is [$($expected:literal),+]) => {
                assert_approx_eq!(
                    $projection.project_all_unchecked(&[$($from),+]).collect::<Vec<_>>(),
                    vec![$($expected),+],
                    epsilon = 1e-6
                );
            };
        }

        assert_project_to!(projection from [0, 0] is [1.00, 0.00, 0.00, 0.00]);
        assert_project_to!(projection from [0, 1] is [0.50, 0.50, 0.00, 0.00]);
        assert_project_to!(projection from [0, 2] is [0.00, 1.00, 0.00, 0.00]);
        assert_project_to!(projection from [1, 0] is [0.50, 0.00, 0.50, 0.00]);
        assert_project_to!(projection from [1, 1] is [0.25, 0.25, 0.25, 0.25]);
        assert_project_to!(projection from [1, 2] is [0.00, 0.50, 0.00, 0.50]);
        assert_project_to!(projection from [2, 0] is [0.00, 0.00, 1.00, 0.00]);
        assert_project_to!(projection from [2, 1] is [0.00, 0.00, 0.50, 0.50]);
        assert_project_to!(projection from [2, 2] is [0.00, 0.00, 0.00, 1.00]);
    }
}
