//! Hypergeometric distribution.
//!
//! Much of the code here is adapted from the implementation in statrs.

use factorial::ln_factorial;

/// Returns the sum of the first n - 1 terms of the harmonic series
pub fn harmonic(n: u64) -> f64 {
    p_harmonic(n, 1)
}

/// Returns the sum of the first n - 1 terms of the p-harmonic series
pub fn p_harmonic(n: u64, p: u32) -> f64 {
    (1..n).map(|i| 1.0 / (i.pow(p) as f64)).sum()
}

/// Returns the PMF of the hypergeometric distribution.
pub fn hypergeometric_pmf(size: u64, successes: u64, draws: u64, observed: u64) -> f64 {
    if observed > draws {
        0.0
    } else {
        binomial(successes, observed) * binomial(size - successes, draws - observed)
            / binomial(size, draws)
    }
}

/// Returns the binomial coefficient.
pub fn binomial(n: u64, k: u64) -> f64 {
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

    const LN_2_SQRT_E_OVER_PI: f64 = 0.620_782_237_635_245_2;
    const LN_PI: f64 = 1.144_729_885_849_400_2;
    const R: f64 = 10.900511;
    const DK: &[f64] = &[
        2.485_740_891_387_535_5e-5,
        1.051_423_785_817_219_7,
        -3.456_870_972_220_162_5,
        4.512_277_094_668_948,
        -2.982_852_253_235_766_4,
        1.056_397_115_771_267,
        -1.954_287_731_916_458_7e-1,
        1.709_705_434_044_412e-2,
        -5.719_261_174_043_057e-4,
        4.633_994_733_599_057e-6,
        -2.719_949_084_886_077_2e-9,
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
    fn test_hypergeometric_pmf() {
        assert_approx_eq!(hypergeometric_pmf(10, 7, 8, 4), 0.0, epsilon = 1e-6);
        assert_approx_eq!(hypergeometric_pmf(10, 7, 8, 5), 0.466667, epsilon = 1e-6);
        assert_approx_eq!(hypergeometric_pmf(10, 7, 8, 6), 0.466667, epsilon = 1e-6);
        assert_approx_eq!(hypergeometric_pmf(10, 7, 8, 7), 0.066667, epsilon = 1e-6);
        assert_approx_eq!(hypergeometric_pmf(10, 7, 8, 8), 0.0, epsilon = 1e-6);

        assert_approx_eq!(hypergeometric_pmf(6, 2, 2, 0), 0.4, epsilon = 1e-6);
        assert_approx_eq!(hypergeometric_pmf(6, 2, 2, 1), 0.533333, epsilon = 1e-6);
        assert_approx_eq!(hypergeometric_pmf(6, 2, 2, 2), 0.066667, epsilon = 1e-6);
    }
}
