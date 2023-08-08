use std::fmt;

use crate::array::Shape;

use super::Scs;

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

    pub fn new<S>(from: S, to: S) -> Result<Self, ProjectionError>
    where
        S: Into<Shape>,
    {
        let from = from.into();
        let to = to.into();

        if from.dimensions() == to.dimensions() {
            if let Some(dimension) = from
                .iter()
                .zip(to.iter())
                .enumerate()
                .find_map(|(i, (from, to))| (from < to).then_some(i))
            {
                Err(ProjectionError::InvalidProjection {
                    dimension,
                    from: from[dimension],
                    to: to[dimension],
                })
            } else {
                Ok(Self::new_unchecked(from, to))
            }
        } else if from.dimensions() == 0 {
            Err(ProjectionError::Empty)
        } else {
            Err(ProjectionError::UnequalDimensions {
                from: from.dimensions(),
                to: to.dimensions(),
            })
        }
    }

    pub fn new_unchecked<S>(from: S, to: S) -> Self
    where
        S: Into<Shape>,
    {
        Self {
            from: from.into(),
            to: to.into(),
        }
    }

    pub fn project<'a>(&'a self, from: &'a [usize]) -> Result<Projected<'a>, ProjectError> {
        if self.dimensions() == from.len() {
            if let Some(dimension) = self
                .to
                .iter()
                .zip(from.iter())
                .enumerate()
                .find_map(|(i, (&shape, &count))| (shape - 1 < count).then_some(i))
            {
                Err(ProjectError::InvalidProjection {
                    dimension,
                    from: from[dimension],
                    to: self.to[dimension],
                })
            } else {
                Ok(self.project_unchecked(from))
            }
        } else {
            Err(ProjectError::UnequalDimensions {
                from: from.len(),
                to: self.dimensions(),
            })
        }
    }

    pub fn project_unchecked<'a>(&'a self, from: &'a [usize]) -> Projected<'a> {
        Projected::new_unchecked(self, from)
    }

    fn project_value_unchecked(&self, from: &[usize], to: &[usize]) -> f64 {
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
}

#[derive(Debug)]
pub struct Projected<'a> {
    iter: ProjectIter<'a>,
    weight: f64,
}

impl<'a> Projected<'a> {
    pub fn add_unchecked(self, to: &mut Scs) {
        to.inner_mut()
            .iter_mut()
            .zip(self.iter)
            .for_each(|(to, projected)| *to += projected * self.weight);
    }

    fn new_unchecked(projection: &'a Projection, from: &'a [usize]) -> Self {
        Self {
            iter: ProjectIter::new_unchecked(projection, from),
            weight: 1.0,
        }
    }

    pub fn into_weighted(mut self, weight: f64) -> Self {
        self.weight = weight;
        self
    }
}

#[derive(Debug)]
struct ProjectIter<'a> {
    projection: &'a Projection,
    from: &'a [usize],
    to: Vec<usize>,
    index: usize,
}

impl<'a> ProjectIter<'a> {
    fn dimensions(&self) -> usize {
        self.to.len()
    }

    fn impl_next_rec(&mut self, axis: usize) -> Option<<Self as Iterator>::Item> {
        if self.index == 0 {
            self.index += 1;
            return Some(self.project_value());
        };

        self.to[axis] += 1;
        if self.to[axis] < self.projection.to[axis] {
            self.index += 1;
            Some(self.project_value())
        } else if axis > 0 {
            self.to[axis] = 0;
            self.impl_next_rec(axis - 1)
        } else {
            None
        }
    }

    fn new_unchecked(projection: &'a Projection, from: &'a [usize]) -> Self {
        Self {
            projection,
            from,
            to: vec![0; from.len()],
            index: 0,
        }
    }

    fn project_value(&self) -> f64 {
        self.projection.project_value_unchecked(self.from, &self.to)
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
    Empty,
    InvalidProjection {
        dimension: usize,
        from: usize,
        to: usize,
    },
    UnequalDimensions {
        from: usize,
        to: usize,
    },
}

impl fmt::Display for ProjectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProjectionError::Empty => f.write_str("cannot project empty shapes"),
            ProjectionError::InvalidProjection {
                dimension,
                from,
                to,
            } => {
                write!(
                    f,
                    "cannot project from shape {from} to shape {to} in dimension {dimension}"
                )
            }
            ProjectionError::UnequalDimensions { from, to } => {
                write!(
                    f,
                    "cannot project from one number of dimensions ({from}) to another ({to})"
                )
            }
        }
    }
}

impl std::error::Error for ProjectionError {}
#[derive(Debug)]
pub enum ProjectError {
    InvalidProjection {
        dimension: usize,
        from: usize,
        to: usize,
    },
    UnequalDimensions {
        from: usize,
        to: usize,
    },
}

impl fmt::Display for ProjectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProjectError::InvalidProjection {
                dimension,
                from,
                to,
            } => {
                write!(
                    f,
                    "cannot project from shape {from} to shape {to} in dimension {dimension}"
                )
            }
            ProjectError::UnequalDimensions { from, to } => {
                write!(
                    f,
                    "cannot project from one number of dimensions ({from}) to another ({to})"
                )
            }
        }
    }
}

impl std::error::Error for ProjectError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_projection_errors() {
        assert!(matches!(
            Projection::new(vec![2, 3], vec![1]),
            Err(ProjectionError::UnequalDimensions { .. })
        ));

        assert!(matches!(
            Projection::new([2, 3], [3, 2]),
            Err(ProjectionError::InvalidProjection { .. })
        ))
    }

    #[test]
    fn test_project_7_to_3_project_2() {
        let projection = Projection::new_unchecked(Shape::from(7), Shape::from(3));

        assert_approx_eq!(
            ProjectIter::new_unchecked(&projection, &[2]).collect::<Vec<_>>(),
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
                    ProjectIter::new_unchecked(&$projection, &[$($from),+]).collect::<Vec<_>>(),
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
