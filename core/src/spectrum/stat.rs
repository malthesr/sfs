use std::fmt;

pub mod theta;
pub use theta::Theta;

pub mod d;
pub use d::D;

use super::{Sfs, Shape, Spectrum, State};

pub type Pi = Theta<theta::Tajima>;

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct PiXY(pub f64);

impl PiXY {
    pub fn from_spectrum<S: State>(spectrum: &Spectrum<S>) -> Result<Self, DimensionError> {
        if spectrum.dimensions() == 2 {
            Ok(Self::from_spectrum_unchecked(spectrum))
        } else {
            Err(DimensionError {
                expected: 2,
                actual: spectrum.dimensions(),
            })
        }
    }

    fn from_spectrum_unchecked<S: State>(spectrum: &Spectrum<S>) -> Self {
        let (n1, n2) = if let &[n1, n2] = spectrum.shape().as_ref() {
            (n1 - 1, n2 - 1)
        } else {
            panic!("dimensions do not fit");
        };

        let num = (0..=n1)
            .flat_map(|m1| (0..=n2).map(move |m2| (m1, m2)))
            .take(spectrum.elements() - 1)
            .skip(1)
            .map(|(m1, m2)| {
                let p1 = m1 * (n2 - m2);
                let p2 = m2 * (n1 - m1);
                spectrum[[m1, m2]] * ((p1 + p2) as f64)
            })
            .sum::<f64>();

        let denom = (n1 * n2) as f64;

        Self(num / denom)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct F2(pub f64);

impl F2 {
    pub fn from_sfs(sfs: &Sfs) -> Result<Self, DimensionError> {
        if sfs.dimensions() == 2 {
            Ok(Self::from_sfs_unchecked(sfs))
        } else {
            Err(DimensionError {
                expected: 2,
                actual: sfs.dimensions(),
            })
        }
    }

    fn from_sfs_unchecked(sfs: &Sfs) -> Self {
        Self(
            sfs.array
                .iter()
                .zip(sfs.iter_frequencies())
                .map(|(v, fs)| v * (fs[0] - fs[1]).powi(2))
                .sum(),
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct F3(pub f64);

impl F3 {
    pub fn from_sfs(sfs: &Sfs) -> Result<Self, DimensionError> {
        if sfs.dimensions() == 3 {
            Ok(Self::from_sfs_unchecked(sfs))
        } else {
            Err(DimensionError {
                expected: 3,
                actual: sfs.dimensions(),
            })
        }
    }

    fn from_sfs_unchecked(sfs: &Sfs) -> Self {
        Self(
            sfs.array
                .iter()
                .zip(sfs.iter_frequencies())
                .map(|(v, fs)| v * (fs[0] - fs[1]) * (fs[0] - fs[2]))
                .sum(),
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct F4(pub f64);

impl F4 {
    pub fn from_sfs(sfs: &Sfs) -> Result<Self, DimensionError> {
        if sfs.dimensions() == 4 {
            Ok(Self::from_sfs_unchecked(sfs))
        } else {
            Err(DimensionError {
                expected: 4,
                actual: sfs.dimensions(),
            })
        }
    }

    fn from_sfs_unchecked(sfs: &Sfs) -> Self {
        Self(
            sfs.array
                .iter()
                .zip(sfs.iter_frequencies())
                .map(|(v, fs)| v * (fs[0] - fs[1]) * (fs[2] - fs[3]))
                .sum(),
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct Fst(pub f64);

impl Fst {
    pub fn from_sfs(sfs: &Sfs) -> Result<Self, DimensionError> {
        if sfs.dimensions() == 2 {
            Ok(Self::from_sfs_unchecked(sfs))
        } else {
            Err(DimensionError {
                expected: 2,
                actual: sfs.dimensions(),
            })
        }
    }

    fn from_sfs_unchecked(sfs: &Sfs) -> Self {
        // We only want the polymorphic parts of the spectrum and corresponding frequencies,
        // so we drop the first and last values
        let polymorphic_iter = sfs
            .array
            .iter()
            .zip(sfs.iter_frequencies())
            .take(sfs.elements() - 1)
            .skip(1);

        let shape = sfs.shape();
        let n_i_sub = (shape[0] - 2) as f64;
        let n_j_sub = (shape[1] - 2) as f64;

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
    pub fn from_spectrum<S: State>(spectrum: &Spectrum<S>) -> Result<Self, ShapeError> {
        if spectrum.shape().0 == [3, 3] {
            Ok(Self::from_spectrum_unchecked(spectrum))
        } else {
            Err(ShapeError {
                expected: Shape(vec![3, 3]),
                actual: spectrum.shape().clone(),
            })
        }
    }

    fn from_spectrum_unchecked<S: State>(spectrum: &Spectrum<S>) -> Self {
        let s = spectrum;

        let numer = s[[1, 1]] - 2. * (s[[0, 2]] + s[[2, 0]]);
        let denom = s[[0, 1]] + s[[1, 0]] + 2. * s[[1, 1]] + s[[1, 2]] + s[[2, 1]];

        Self(numer / denom)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct R0(pub f64);

impl R0 {
    pub fn from_spectrum<S: State>(spectrum: &Spectrum<S>) -> Result<Self, ShapeError> {
        if spectrum.shape().0 == [3, 3] {
            Ok(Self::from_spectrum_unchecked(spectrum))
        } else {
            Err(ShapeError {
                expected: Shape(vec![3, 3]),
                actual: spectrum.shape().clone(),
            })
        }
    }

    fn from_spectrum_unchecked<S: State>(spectrum: &Spectrum<S>) -> Self {
        let s = spectrum;

        Self((s[[0, 2]] + s[[2, 0]]) / s[[1, 1]])
    }
}

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct R1(pub f64);

impl R1 {
    pub fn from_spectrum<S: State>(spectrum: &Spectrum<S>) -> Result<Self, ShapeError> {
        if spectrum.shape().0 == [3, 3] {
            Ok(Self::from_spectrum_unchecked(spectrum))
        } else {
            Err(ShapeError {
                expected: Shape(vec![3, 3]),
                actual: spectrum.shape().clone(),
            })
        }
    }

    fn from_spectrum_unchecked<S: State>(spectrum: &Spectrum<S>) -> Self {
        let denom = [[0, 1], [0, 2], [1, 0], [1, 2], [2, 0], [2, 1]]
            .iter()
            .map(|&i| spectrum[i])
            .sum::<f64>();
        Self(spectrum[[1, 1]] / denom)
    }
}

/// An error associated with the calculation of a statistic from a spectrum.
#[derive(Debug)]
pub enum StatisticError {
    /// The statistic is not defined for an array of the provided dimensionality.
    DimensionError(DimensionError),
    /// The statistic is not defined for an array of the provided shape.
    ShapeError(ShapeError),
}

impl fmt::Display for StatisticError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StatisticError::DimensionError(e) => write!(f, "{e}"),
            StatisticError::ShapeError(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for StatisticError {}

impl From<ShapeError> for StatisticError {
    fn from(e: ShapeError) -> Self {
        Self::ShapeError(e)
    }
}

impl From<DimensionError> for StatisticError {
    fn from(e: DimensionError) -> Self {
        Self::DimensionError(e)
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

#[cfg(test)]
mod tests {
    use crate::Scs;

    #[test]
    fn test_pi_xy() {
        let data = vec![
            0.983094, 0.000819, 0.000298, 0.001926, 0.000439, 0.000266, 0.000753, 0.000380,
            0.000330, 0.000256, 0.000217, 0.000398, 0.000102, 0.000166, 0.010558,
        ];

        let sfs = Scs::new(data, [5, 3]).unwrap();

        assert_approx_eq!(sfs.pi_xy().unwrap(), 0.002925);
    }
}
