//! Hypergeometric distribution.
//!
//! Much of the code here is adapted from the implementation in statrs.

use factorial::ln_factorial;

pub fn pmf_unchecked(size: u64, successes: u64, draws: u64, observed: u64) -> f64 {
    if observed > draws {
        0.0
    } else {
        binomial(successes, observed) * binomial(size - successes, draws - observed)
            / binomial(size, draws)
    }
}

fn binomial(n: u64, k: u64) -> f64 {
    if k > n {
        0.0
    } else {
        (0.5 + (ln_factorial(n) - ln_factorial(k) - ln_factorial(n - k)).exp()).floor()
    }
}

mod factorial {
    use std::sync::OnceLock;

    use super::gamma::ln_gamma;

    const MAX: usize = 170;
    const PRECOMPUTED_LEN: usize = MAX + 1;

    fn precomputed() -> &'static [f64; PRECOMPUTED_LEN] {
        static PRECOMPUTED: OnceLock<[f64; PRECOMPUTED_LEN]> = OnceLock::new();

        PRECOMPUTED.get_or_init(|| {
            let mut precomputed = [1.0; PRECOMPUTED_LEN];

            precomputed
                .iter_mut()
                .enumerate()
                .skip(1)
                .fold(1.0, |acc, (i, x)| {
                    let factorial = acc * i as f64;
                    *x = factorial;
                    factorial
                });

            precomputed
        })
    }

    pub(super) fn ln_factorial(x: u64) -> f64 {
        precomputed()
            .get(x as usize)
            .map(|factorial| factorial.ln())
            .unwrap_or_else(|| ln_gamma(x as f64 + 1.0))
    }
}

mod gamma {
    use std::f64::consts::{E, PI};

    const LN_2_SQRT_E_OVER_PI: f64 = 0.6207822376352452223455184457816472122518527279025978;
    const LN_PI: f64 = 1.1447298858494001741434273513530587116472948129153;
    const R: f64 = 10.900511;
    const DK: &[f64] = &[
        2.48574089138753565546e-5,
        1.05142378581721974210,
        -3.45687097222016235469,
        4.51227709466894823700,
        -2.98285225323576655721,
        1.05639711577126713077,
        -1.95428773191645869583e-1,
        1.70970543404441224307e-2,
        -5.71926117404305781283e-4,
        4.63399473359905636708e-6,
        -2.71994908488607703910e-9,
    ];

    pub(super) fn ln_gamma(x: f64) -> f64 {
        if x < 0.5 {
            let s = DK
                .iter()
                .enumerate()
                .skip(1)
                .fold(DK[0], |s, t| s + t.1 / (t.0 as f64 - x));

            LN_PI
                - (PI * x).sin().ln()
                - s.ln()
                - LN_2_SQRT_E_OVER_PI
                - (0.5 - x) * ((0.5 - x + R) / E).ln()
        } else {
            let s = DK
                .iter()
                .enumerate()
                .skip(1)
                .fold(DK[0], |s, t| s + t.1 / (x + t.0 as f64 - 1.0));

            s.ln() + LN_2_SQRT_E_OVER_PI + (x - 0.5) * ((x - 0.5 + R) / E).ln()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hypergeometric() {
        assert_approx_eq!(pmf_unchecked(10, 7, 8, 4), 0.0, epsilon = 1e-6);
        assert_approx_eq!(pmf_unchecked(10, 7, 8, 5), 0.466667, epsilon = 1e-6);
        assert_approx_eq!(pmf_unchecked(10, 7, 8, 6), 0.466667, epsilon = 1e-6);
        assert_approx_eq!(pmf_unchecked(10, 7, 8, 7), 0.066667, epsilon = 1e-6);
        assert_approx_eq!(pmf_unchecked(10, 7, 8, 8), 0.0, epsilon = 1e-6);

        assert_approx_eq!(pmf_unchecked(6, 2, 2, 0), 0.4, epsilon = 1e-6);
        assert_approx_eq!(pmf_unchecked(6, 2, 2, 1), 0.533333, epsilon = 1e-6);
        assert_approx_eq!(pmf_unchecked(6, 2, 2, 2), 0.066667, epsilon = 1e-6);
    }
}