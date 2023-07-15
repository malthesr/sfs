use crate::{array::Shape, Scs};

mod hypergeometric;
use hypergeometric::{Distribution, DistributionError, JointIndependentDistribution};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Projection {
    distribution: JointIndependentDistribution,
    coords: Vec<usize>,
}

impl Projection {
    pub fn dimensions(&self) -> usize {
        self.coords.len()
    }

    fn iter_shape(&self) -> impl Iterator<Item = usize> + '_ {
        self.distribution.distributions.iter().map(|d| d.draws)
    }

    pub fn new(from: &Shape, to: &Shape) -> Result<Self, ProjectionError> {
        if from.dimensions() == to.dimensions() {
            let distribution = from
                .iter()
                .zip(to.iter())
                .map(|(&from, &to)| Distribution::new(from - 1, to - 1))
                .collect::<Result<Vec<_>, _>>()
                .and_then(JointIndependentDistribution::new)?;

            let coords = vec![0; from.len()];

            Ok(Self {
                distribution,
                coords,
            })
        } else {
            Err(ProjectionError::MismatchingDimensions)
        }
    }

    fn project(&mut self, count: &[usize]) -> Result<ProjectIter<'_>, ProjectionError> {
        self.distribution.set_successes(count)?;
        self.coords.iter_mut().for_each(|x| *x = 0);

        Ok(ProjectIter {
            distribution: &mut self.distribution,
            coords: &mut self.coords,
            index: 0,
        })
    }

    pub fn project_to(&mut self, count: &[usize], to: &mut Scs) -> Result<(), ProjectionError> {
        self.project_to_weighted(count, to, 1.0)
    }

    pub fn project_to_weighted(
        &mut self,
        count: &[usize],
        to: &mut Scs,
        weight: f64,
    ) -> Result<(), ProjectionError> {
        let shapes_match = self
            .iter_shape()
            .zip(to.shape().iter())
            .all(|(x, &y)| x == y - 1);

        if shapes_match {
            to.inner_mut()
                .iter_mut()
                .zip(self.project(count)?)
                .for_each(|(to, projected)| *to += projected * weight);

            Ok(())
        } else {
            Err(ProjectionError::MismatchingShapes)
        }
    }
}

#[derive(Debug)]
struct ProjectIter<'a> {
    distribution: &'a JointIndependentDistribution,
    coords: &'a mut [usize],
    index: usize,
}

impl<'a> ProjectIter<'a> {
    fn dimensions(&self) -> usize {
        self.coords.len()
    }

    fn index_shape(&self, axis: usize) -> usize {
        self.distribution.distributions[axis].draws + 1
    }

    fn impl_next_rec(&mut self, axis: usize) -> Option<<Self as Iterator>::Item> {
        if self.index == 0 {
            self.index += 1;
            return Some(self.distribution.pmf(self.coords).unwrap());
        };

        self.coords[axis] += 1;
        if self.coords[axis] < self.index_shape(axis) {
            self.index += 1;
            Some(self.distribution.pmf(self.coords).unwrap())
        } else if axis > 0 {
            self.coords[axis] = 0;
            self.impl_next_rec(axis - 1)
        } else {
            None
        }
    }
}

impl<'a> Iterator for ProjectIter<'a> {
    type Item = f64;

    fn next(&mut self) -> Option<Self::Item> {
        self.impl_next_rec(self.dimensions() - 1)
    }
}

#[derive(Debug)]
pub enum ProjectionError {
    InvalidProjection,
    MismatchingDimensions,
    MismatchingShapes,
}

impl From<DistributionError> for ProjectionError {
    fn from(error: DistributionError) -> Self {
        match error {
            DistributionError::WrongNumberOfObservations => Self::MismatchingDimensions,
            DistributionError::DrawsGreaterThanSize
            | DistributionError::Empty
            | DistributionError::SuccessesGreaterThanSize
            | DistributionError::MissingDraws
            | DistributionError::WrongNumberOfSuccesses => Self::InvalidProjection,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_project_7_to_3_project_2() {
        let mut scs = Scs::from_zeros(3);
        let mut projection = Projection::new(&Shape::from(7), scs.shape()).unwrap();

        projection.project_to(&[2], &mut scs).unwrap();
        assert_approx_eq!(
            scs.inner().as_slice(),
            [0.4, 0.533333, 0.066667].as_slice(),
            epsilon = 1e-6
        );
    }

    #[test]
    fn test_project_3x3_to_2x2() {
        let mut scs = Scs::from_zeros([2, 2]);
        let mut projection = Projection::new(&Shape::from([3, 3]), scs.shape()).unwrap();

        macro_rules! assert_project_to {
            ($projection:ident with count [$($count:literal),+] to $scs:ident is [$($expected:literal),+]) => {
                $projection.project_to(&[$($count),+], &mut $scs).unwrap();
                assert_approx_eq!(
                    $scs.inner().as_slice(),
                    [$($expected),+].as_slice(),
                    epsilon = 1e-6
                );
            };
        }

        assert_project_to!(projection with count [0, 0] to scs is [1.00, 0.00, 0.00, 0.00]);
        assert_project_to!(projection with count [0, 1] to scs is [1.50, 0.50, 0.00, 0.00]);
        assert_project_to!(projection with count [0, 2] to scs is [1.50, 1.50, 0.00, 0.00]);
        assert_project_to!(projection with count [1, 0] to scs is [2.00, 1.50, 0.50, 0.00]);
        assert_project_to!(projection with count [1, 1] to scs is [2.25, 1.75, 0.75, 0.25]);
        assert_project_to!(projection with count [1, 2] to scs is [2.25, 2.25, 0.75, 0.75]);
        assert_project_to!(projection with count [2, 0] to scs is [2.25, 2.25, 1.75, 0.75]);
        assert_project_to!(projection with count [2, 1] to scs is [2.25, 2.25, 2.25, 1.25]);
        assert_project_to!(projection with count [2, 2] to scs is [2.25, 2.25, 2.25, 2.25]);
    }
}
