use std::fmt;

use crate::{NormSfs, Sfs, Shape};

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct F2(pub f64);

impl F2 {
    pub fn from_sfs(sfs: &NormSfs) -> Result<Self, DimensionError> {
        if sfs.dimensions() == 2 {
            Ok(Self::from_sfs_unchecked(sfs))
        } else {
            Err(DimensionError {
                expected: 2,
                actual: sfs.dimensions(),
            })
        }
    }

    fn from_sfs_unchecked(sfs: &NormSfs) -> Self {
        Self(
            sfs.iter()
                .zip(sfs.iter_frequencies())
                .map(|(v, fs)| v * (fs[0] - fs[1]).powi(2))
                .sum(),
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct Fst(pub f64);

impl Fst {
    pub fn from_sfs(sfs: &NormSfs) -> Result<Self, DimensionError> {
        if sfs.dimensions() == 2 {
            Ok(Self::from_sfs_unchecked(sfs))
        } else {
            Err(DimensionError {
                expected: 2,
                actual: sfs.dimensions(),
            })
        }
    }

    fn from_sfs_unchecked(sfs: &NormSfs) -> Self {
        // We only want the polymorphic parts of the spectrum and corresponding frequencies,
        // so we drop the first and last values
        let polymorphic_iter = sfs
            .iter()
            .zip(sfs.iter_frequencies())
            .take(sfs.as_slice().len() - 1)
            .skip(1);

        let n_i_sub = (sfs.shape()[0] - 2) as f64;
        let n_j_sub = (sfs.shape()[1] - 2) as f64;

        let (num, denom) = polymorphic_iter
            .map(|(v, fs)| {
                let f_i = fs[0];
                let f_j = fs[1];
                let g_i = 1. - f_i;
                let g_j = 1. - f_j;

                let num = (f_i - f_j).powi(2) - f_i * g_i / n_i_sub - f_j * g_j / n_j_sub;
                let denom = f_i * g_j + f_j * g_i;
                (v * num, v * denom)
            })
            .fold((0., 0.), |(n_sum, d_sum), (n, d)| (n_sum + n, d_sum + d));

        Self(num / denom)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct King(pub f64);

impl King {
    pub fn from_sfs<const N: bool>(sfs: &Sfs<N>) -> Result<Self, ShapeError> {
        if sfs.shape().0 == [3, 3] {
            Ok(Self::from_sfs_unchecked(sfs))
        } else {
            Err(ShapeError {
                expected: Shape(vec![3, 3]),
                actual: sfs.shape().clone(),
            })
        }
    }

    fn from_sfs_unchecked<const N: bool>(sfs: &Sfs<N>) -> Self {
        let numer = sfs[[1, 1]] - 2. * (sfs[[0, 2]] + sfs[[2, 0]]);
        let denom = sfs[[0, 1]] + sfs[[1, 0]] + 2. * sfs[[1, 1]] + sfs[[1, 2]] + sfs[[2, 1]];

        Self(numer / denom)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct R0(pub f64);

impl R0 {
    pub fn from_sfs<const N: bool>(sfs: &Sfs<N>) -> Result<Self, ShapeError> {
        if sfs.shape().0 == [3, 3] {
            Ok(Self::from_sfs_unchecked(sfs))
        } else {
            Err(ShapeError {
                expected: Shape(vec![3, 3]),
                actual: sfs.shape().clone(),
            })
        }
    }

    fn from_sfs_unchecked<const N: bool>(sfs: &Sfs<N>) -> Self {
        Self((sfs[[0, 2]] + sfs[[2, 0]]) / sfs[[1, 1]])
    }
}

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct R1(pub f64);

impl R1 {
    pub fn from_sfs<const N: bool>(sfs: &Sfs<N>) -> Result<Self, ShapeError> {
        if sfs.shape().0 == [3, 3] {
            Ok(Self::from_sfs_unchecked(sfs))
        } else {
            Err(ShapeError {
                expected: Shape(vec![3, 3]),
                actual: sfs.shape().clone(),
            })
        }
    }

    fn from_sfs_unchecked<const N: bool>(sfs: &Sfs<N>) -> Self {
        let denom = [[0, 1], [0, 2], [1, 0], [1, 2], [2, 0], [2, 1]]
            .iter()
            .map(|&i| sfs[i])
            .sum::<f64>();
        Self(sfs[[1, 1]] / denom)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct Heterozygosity(pub f64);

impl Heterozygosity {
    pub fn from_sfs(sfs: &NormSfs) -> Result<Self, ShapeError> {
        if sfs.shape().0 == [3] {
            Ok(Self::from_sfs_unchecked(sfs))
        } else {
            Err(ShapeError {
                expected: Shape(vec![3]),
                actual: sfs.shape().clone(),
            })
        }
    }

    fn from_sfs_unchecked(sfs: &NormSfs) -> Self {
        Self(sfs[[1]])
    }
}

#[derive(Debug)]
pub struct DimensionError {
    expected: usize,
    actual: usize,
}

impl fmt::Display for DimensionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let &DimensionError { expected, actual } = self;
        write!(
            f,
            "expected SFS with dimension {expected}, found SFS with dimension {actual}"
        )
    }
}

impl std::error::Error for DimensionError {}

#[derive(Debug)]
pub struct ShapeError {
    expected: Shape,
    actual: Shape,
}

impl fmt::Display for ShapeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let &ShapeError { expected, actual } = &self;
        write!(
            f,
            "expected SFS with shape {expected}, found SFS with shape {actual}"
        )
    }
}

impl std::error::Error for ShapeError {}
