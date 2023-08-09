use std::fmt;

use super::{Count, Scs};

mod hypergeometric;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PartialProjection {
    project_to: Count,
}

impl PartialProjection {
    pub fn dimensions(&self) -> usize {
        self.project_to.dimensions()
    }

    pub fn new_unchecked<C>(project_to: C) -> Self
    where
        C: Into<Count>,
    {
        Self {
            project_to: project_to.into(),
        }
    }

    pub fn project_to(&self) -> &Count {
        &self.project_to
    }

    pub fn project_unchecked<'a>(
        &'a mut self,
        project_from: &'a Count,
        from: &'a Count,
    ) -> Projected<'a> {
        Projected::new_unchecked(project_from, &self.project_to, from)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Projection {
    project_from: Count,
    inner: PartialProjection,
}

impl Projection {
    pub fn dimensions(&self) -> usize {
        self.inner.dimensions()
    }

    pub fn new<C>(from: C, to: C) -> Result<Self, ProjectionError>
    where
        C: Into<Count>,
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

    pub fn new_unchecked<C>(project_from: C, project_to: C) -> Self
    where
        C: Into<Count>,
    {
        Self {
            project_from: project_from.into(),
            inner: PartialProjection::new_unchecked(project_to),
        }
    }

    pub fn project_to(&self) -> &Count {
        self.inner.project_to()
    }

    pub fn project_unchecked<'a>(&'a mut self, from: &'a Count) -> Projected<'a> {
        self.inner.project_unchecked(&self.project_from, from)
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

    fn new_unchecked(project_from: &'a Count, project_to: &'a Count, from: &'a Count) -> Self {
        Self {
            iter: ProjectIter::new_unchecked(project_from, project_to, from),
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
    project_from: &'a Count,
    project_to: &'a Count,
    from: &'a Count,
    to: Count,
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
        if self.to[axis] <= self.project_to[axis] {
            self.index += 1;
            Some(self.project_value())
        } else if axis > 0 {
            self.to[axis] = 0;
            self.impl_next_rec(axis - 1)
        } else {
            None
        }
    }

    fn new_unchecked(project_from: &'a Count, project_to: &'a Count, from: &'a Count) -> Self {
        Self {
            project_from,
            project_to,
            from,
            to: Count::from_zeros(from.len()),
            index: 0,
        }
    }

    fn project_value(&self) -> f64 {
        self.project_from
            .iter()
            .zip(self.from.iter())
            .zip(self.project_to.iter())
            .zip(self.to.iter())
            .map(|(((&size, &successes), &draws), &observed)| {
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
            ProjectionError::Empty => f.write_str("cannot project empty counts"),
            ProjectionError::InvalidProjection {
                dimension,
                from,
                to,
            } => {
                write!(
                    f,
                    "cannot project from count {from} to count {to} in dimension {dimension}"
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
                    "cannot project from count {from} to count {to} in dimension {dimension}"
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
        let projection = Projection::new_unchecked(Count::from(6), Count::from(2));

        assert_approx_eq!(
            ProjectIter::new_unchecked(
                &projection.project_from,
                &projection.inner.project_to,
                &Count::from(2)
            )
            .collect::<Vec<_>>(),
            vec![0.4, 0.533333, 0.066667],
            epsilon = 1e-6
        );
    }

    #[test]
    fn test_project_3x3_to_2x2() {
        let projection = Projection::new_unchecked(Count::from([2, 2]), Count::from([1, 1]));

        macro_rules! assert_project_to {
            ($projection:ident from [$($from:literal),+] is [$($expected:literal),+]) => {
                assert_approx_eq!(
                    ProjectIter::new_unchecked(&$projection.project_from, &$projection.inner.project_to, &Count::from([$($from),+])).collect::<Vec<_>>(),
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
