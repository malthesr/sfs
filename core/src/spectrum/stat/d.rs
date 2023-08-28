use std::marker::PhantomData;

use crate::{
    spectrum::stat::theta::ThetaEstimator,
    utils::{harmonic, p_harmonic},
    Scs,
};

use super::{theta, DimensionError, Theta};

mod private {
    use super::*;

    pub trait Statistic {
        type T1: ThetaEstimator;
        type T2: ThetaEstimator;

        fn variance(scs: &Scs) -> f64;

        fn estimate_unchecked(scs: &Scs) -> f64 {
            let t1 = Theta::<Self::T1>::from_spectrum_unchecked(scs).0;
            let t2 = Theta::<Self::T2>::from_spectrum_unchecked(scs).0;
            let var = Self::variance(scs);

            (dbg!(t1) - dbg!(t2)) / var
        }
    }
}

#[non_exhaustive]
pub struct FuLi;

impl private::Statistic for FuLi {
    type T1 = theta::Watterson;
    type T2 = theta::FuLi;

    fn variance(scs: &Scs) -> f64 {
        // Notation from Fu and Li (1993), see also Durrett (2008), p. 67, though we use thetas
        // in the numerator here, so there's an extra factor 1/a in the denominator
        let n = scs.elements();
        let s = scs.segregating_sites();

        let a = harmonic(n as u64);
        let g = p_harmonic(n as u64, 2);

        let c_num = 2.0 * n as f64 * a - ((4 * (n - 1)) as f64);
        let c_denom = ((n - 1) * (n - 2)) as f64;
        let c = c_num / c_denom;

        let v = 1.0 + a.powi(2) / (g + a.powi(2)) * (c - ((n + 1) as f64 / (n - 1) as f64));
        let u = a - 1.0 - v;

        (u * s + v * s.powi(2)).sqrt() / a
    }
}

#[non_exhaustive]
pub struct Tajima;

impl private::Statistic for Tajima {
    type T1 = theta::Tajima;
    type T2 = theta::Watterson;

    fn variance(scs: &Scs) -> f64 {
        // Notation from Tajima (1989), see also Durrett (2008), pp. 65-66
        let n = scs.elements();
        let s = scs.segregating_sites();

        let a1 = harmonic(n as u64);
        let a2 = p_harmonic(n as u64, 2);

        let b1 = (n + 1) as f64 / (3 * (n - 1)) as f64;
        let b2 = (2 * (n.pow(2) + n + 3)) as f64 / (9 * n * (n - 1)) as f64;

        let c1 = b1 - 1.0 / a1;
        let c2 = b2 - (n + 2) as f64 / (a1 * n as f64) + a2 / a1.powi(2);

        let e1 = c1 / a1;
        let e2 = c2 / (a1.powi(2) + a2);

        (e1 * s + e2 * s * (s - 1.0)).sqrt()
    }
}

pub trait DStatistic: private::Statistic {}
impl<T> DStatistic for T where T: private::Statistic {}

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct D<S>(pub f64, PhantomData<S>)
where
    S: DStatistic;

impl<S> D<S>
where
    S: DStatistic,
{
    pub fn from_scs(scs: &Scs) -> Result<Self, DimensionError> {
        if scs.dimensions() == 1 {
            Ok(Self::from_spectrum_unchecked(scs))
        } else {
            Err(DimensionError {
                expected: 1,
                actual: scs.dimensions(),
            })
        }
    }

    fn from_spectrum_unchecked(scs: &Scs) -> Self {
        Self(S::estimate_unchecked(scs), PhantomData)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::spectrum::stat::theta::tests::{scs_aquadro, scs_hamblin, scs_hamblin_mod};

    #[test]
    fn test_tajima_d_aquadro() {
        assert_approx_eq!(D::<Tajima>::from_scs(&scs_aquadro()).unwrap().0, -0.995875);
    }

    #[test]
    fn test_tajima_d_hamblin() {
        assert_approx_eq!(D::<Tajima>::from_scs(&scs_hamblin()).unwrap().0, 0.885737);
    }

    #[test]
    fn test_fu_li_d_hamblin() {
        // Durrett gives 1.68, the difference is due to rounding errors in the text
        assert_approx_eq!(D::<FuLi>::from_scs(&scs_hamblin_mod()).unwrap().0, 1.693537);
    }
}
