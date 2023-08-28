use std::marker::PhantomData;

use crate::{
    spectrum::State,
    utils::{binomial, harmonic},
    Spectrum,
};

use super::DimensionError;

mod private {
    use super::*;

    pub trait Estimator {
        fn weight(i: usize, n: usize) -> f64;

        fn estimate_unchecked<S: State>(spectrum: &Spectrum<S>) -> f64 {
            let n = spectrum.elements();

            spectrum
                .array
                .iter()
                .enumerate()
                .take(n)
                .skip(1)
                .map(|(i, &v)| Self::weight(i, n) * v)
                .sum()
        }
    }
}

pub trait ThetaEstimator: private::Estimator {}
impl<T> ThetaEstimator for T where T: private::Estimator {}

#[non_exhaustive]
pub struct FuLi;

impl private::Estimator for FuLi {
    fn estimate_unchecked<S: State>(spectrum: &Spectrum<S>) -> f64 {
        spectrum.inner().as_slice()[1]
    }

    fn weight(_: usize, _: usize) -> f64 {
        unimplemented!()
    }
}

#[non_exhaustive]
pub struct Tajima;

impl private::Estimator for Tajima {
    #[inline]
    fn weight(i: usize, n: usize) -> f64 {
        (i * (n - i)) as f64 / binomial(n as u64, 2)
    }
}

#[non_exhaustive]
pub struct Watterson;

impl private::Estimator for Watterson {
    #[inline]
    fn weight(_: usize, n: usize) -> f64 {
        // We're relying on this to be inlined and hoisted out as loop-invariant for this not to
        // be inefficient; even if it isn't, this is unlikely to ever be a bottleneck anywhere
        1.0 / harmonic(n as u64)
    }
}

#[non_exhaustive]
pub struct FayWu;

impl private::Estimator for FayWu {
    fn weight(i: usize, n: usize) -> f64 {
        binomial(n as u64, 2) * i.pow(2) as f64
    }
}

/// An estimate of θ based on a particular estimator.
///
/// The spectrum may be in frequencies or counts, which corresponds to the estimate of θ being per
/// base or not, respectively.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct Theta<E>(pub f64, PhantomData<E>)
where
    E: ThetaEstimator;

impl<E> Theta<E>
where
    E: ThetaEstimator,
{
    pub fn from_spectrum<S: State>(spectrum: &Spectrum<S>) -> Result<Self, DimensionError> {
        if spectrum.dimensions() == 1 {
            Ok(Self::from_spectrum_unchecked(spectrum))
        } else {
            Err(DimensionError {
                expected: 1,
                actual: spectrum.dimensions(),
            })
        }
    }

    pub(super) fn from_spectrum_unchecked<S: State>(spectrum: &Spectrum<S>) -> Self {
        Self(E::estimate_unchecked(spectrum), PhantomData)
    }
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;

    use crate::Scs;

    // Recreating the spectrum based on the data from Ward et al. (1991) in Durrett (2008) p. 30 -
    // or, rather, the one that Durrett counts, on p. 40, though his counts are off
    pub fn scs_ward() -> Scs {
        const COUNTS: [(usize, usize); 13] = [
            (1, 6),
            (2, 2),
            (3, 3),
            (4, 1),
            (6, 4),
            (7, 1),
            (10, 1),
            (12, 2),
            (13, 1),
            (23, 1),
            (24, 1),
            (25, 1),
            (28, 2),
        ];
        const SITES: usize = 360;

        let mut scs = Scs::from_zeros(63);
        for (i, v) in COUNTS {
            scs[[i]] = v as f64;
        }

        scs[[0]] = SITES as f64 - scs.sum();
        scs
    }

    fn scs_from_counts(counts: &[usize]) -> Scs {
        let mut scs = Scs::from_zeros(counts.len());
        for (i, &v) in counts.iter().enumerate() {
            scs[[i]] = v as f64;
        }

        scs
    }

    // Recreating the spectrum based on the data from Aquadro and Greenberg (1983)
    // in Durrett (2008) p. 44
    pub fn scs_aquadro() -> Scs {
        const COUNTS: [usize; 7] = [0, 34, 6, 4, 0, 0, 0];
        scs_from_counts(&COUNTS)
    }

    // Recreating the spectrum based on the data from Hamblin and Aquadro (1996)
    // in Durrett (2008) p. 68 (without multiallelics as listed on p. 69)
    pub fn scs_hamblin() -> Scs {
        const COUNTS: [usize; 11] = [0, 1, 11, 4, 7, 2, 0, 0, 0, 0, 0];
        scs_from_counts(&COUNTS)
    }

    // Recreating the spectrum based on the data from Hamblin and Aquadro (1996)
    // in Durrett (2008) p. 68
    pub fn scs_hamblin_mod() -> Scs {
        let mut scs = scs_hamblin();
        scs[[8]] += 1.0;
        scs[[3]] += 1.0;
        scs
    }

    #[test]
    fn test_theta_watterson_ward() {
        assert_approx_eq!(
            Theta::<Watterson>::from_spectrum(&scs_ward()).unwrap().0,
            5.517367
        );
    }

    #[test]
    fn test_theta_tajima_ward() {
        assert_approx_eq!(
            Theta::<Tajima>::from_spectrum(&scs_ward()).unwrap().0,
            5.285202
        );
    }

    #[test]
    fn test_theta_watterson_aquadro() {
        assert_approx_eq!(
            Theta::<Watterson>::from_spectrum(&scs_aquadro()).unwrap().0,
            17.959184
        );
    }

    #[test]
    fn test_theta_tajima_aquadro() {
        assert_approx_eq!(
            Theta::<Tajima>::from_spectrum(&scs_aquadro()).unwrap().0,
            14.857143
        );
    }
}
