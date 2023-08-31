use std::{cmp::Ordering, marker::PhantomData};

use crate::Array;

use super::{Scs, Spectrum, State};

/// A folded spectrum.
#[derive(Debug, PartialEq)]
pub struct Folded<S: State> {
    array: Array<Option<f64>>, // "lower" triangle is None,
    state: PhantomData<S>,
}

impl<S: State> Folded<S> {
    pub(super) fn from_spectrum(spectrum: &Spectrum<S>) -> Self {
        let n = spectrum.elements();
        let total_count = spectrum.shape().iter().sum::<usize>() - spectrum.shape().len();

        // In general, this point divides the folding line. Since we are folding onto the "upper"
        // part of the array, we want to fold anything "below" it onto something "above" it.
        let mid_count = total_count / 2;

        // The spectrum may or may not have a "diagonal", i.e. a hyperplane that falls exactly on
        // the midpoint. If such a diagonal exists, we need to handle it as a special case when
        // folding below.
        //
        // For example, in 1D a spectrum with five elements has a "diagonal", marked X:
        // [-, -, X, -, -]
        // Whereas on with four elements would not.
        //
        // In two dimensions, e.g. three-by-three elements has a diagonal:
        // [-, -, X]
        // [-, X, -]
        // [X, -, -]
        // whereas two-by-three would not. On the other hand, two-by-four has a diagonal:
        // [-, -, X, -]
        // [-, X, -, -]
        //
        // Note that even-ploidy data should always have a diagonal, whereas odd-ploidy data
        // may or may not.
        let has_diagonal = total_count % 2 == 0;

        // Note that we cannot use the algorithm below in-place, since the reverse iterator
        // may reach elements that have already been folded, which causes bugs. Hence we fold
        // into a zero-initialised copy.
        let mut array = Array::from_element(None, spectrum.shape().clone());

        // We iterate over indices rather than values since we have to mutate on the array
        // while looking at it from both directions.
        (0..n).zip((0..n).rev()).for_each(|(i, rev_i)| {
            let count = spectrum.shape().index_sum_from_flat_unchecked(i);

            let src = spectrum.array.as_slice();
            let dst = array.as_mut_slice();

            match (count.cmp(&mid_count), has_diagonal) {
                (Ordering::Less, _) | (Ordering::Equal, false) => {
                    // We are in the upper part of the spectrum that should be folded onto.
                    dst[i] = Some(src[i] + src[rev_i]);
                }
                (Ordering::Equal, true) => {
                    // We are on a diagonal, which must be handled as a special case:
                    // there are apparently different opinions on what the most correct
                    // thing to do is. This adopts the same strategy as e.g. in dadi.
                    dst[i] = Some(0.5 * src[i] + 0.5 * src[rev_i]);
                }
                (Ordering::Greater, _) => {
                    // We are in the lower part of the spectrum to be filled with None;
                    dst[i] = None;
                }
            }
        });

        Self {
            array,
            state: PhantomData,
        }
    }

    /// Returns an unfolded spectrum based on the folded spectrum, filling the folded elements with
    /// the provided element.
    pub fn into_spectrum(&self, fill: f64) -> Spectrum<S> {
        let data = Vec::from_iter(self.array.iter().map(|x| x.unwrap_or(fill)));
        let shape = self.array.shape().clone();
        let array = Array::new_unchecked(data, shape);

        Scs::from(array).into_state_unchecked()
    }
}

impl<S: State> Clone for Folded<S> {
    fn clone(&self) -> Self {
        Self {
            array: self.array.clone(),
            state: PhantomData,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fold_4() {
        let scs = Scs::from_range(0..4, 4).unwrap();
        let expected = Scs::new([3., 3., 0., 0.], 4).unwrap();
        assert_eq!(scs.fold().into_spectrum(0.0), expected);
    }

    #[test]
    fn test_fold_5() {
        let scs = Scs::from_range(0..5, 5).unwrap();
        let expected = Scs::new([4., 4., 2., -1., -1.], 5).unwrap();
        assert_eq!(scs.fold().into_spectrum(-1.), expected);
    }

    #[test]
    fn test_fold_3x3() {
        let scs = Scs::from_range(0..9, [3, 3]).unwrap();

        #[rustfmt::skip]
        let expected = Scs::new(
            [
                8., 8., 4.,
                8., 4., 0.,
                4., 0., 0.,
            ],
            [3, 3]
        ).unwrap();

        assert_eq!(scs.fold().into_spectrum(0.0), expected);
    }

    #[test]
    fn test_fold_2x4() {
        let scs = Scs::from_range(0..8, [2, 4]).unwrap();

        #[rustfmt::skip]
        let expected = Scs::new(
            [
                7., 7.,            3.5, f64::INFINITY,
                7., 3.5, f64::INFINITY, f64::INFINITY,
            ],
            [2, 4]
        ).unwrap();

        assert_eq!(scs.fold().into_spectrum(f64::INFINITY), expected);
    }

    #[test]
    fn test_fold_3x4() {
        let scs = Scs::from_range(0..12, [3, 4]).unwrap();

        #[rustfmt::skip]
        let expected = Scs::new(
            [
                11., 11., 11., 0.,
                11., 11.,  0., 0.,
                11.,  0.,  0., 0.,
            ],
            [3, 4]
        ).unwrap();

        assert_eq!(scs.fold().into_spectrum(0.), expected);
    }

    #[test]
    fn test_fold_3x7() {
        let scs = Scs::from_range(0..21, [3, 7]).unwrap();

        #[rustfmt::skip]
        let expected = Scs::new(
            [
                20., 20., 20., 20., 10., 0., 0.,
                20., 20., 20., 10.,  0., 0., 0.,
                20., 20., 10.,  0.,  0., 0., 0.,
            ],
            [3, 7]
        ).unwrap();

        assert_eq!(scs.fold().into_spectrum(0.0), expected);
    }

    #[test]
    fn test_fold_2x2x2() {
        let scs = Scs::from_range(0..8, [2, 2, 2]).unwrap();

        #[rustfmt::skip]
        let expected = Scs::new(
            [
                 7.,  7.,
                 7., -1.,
                
                 7., -1.,
                -1., -1.,
            ],
            [2, 2, 2]
        ).unwrap();

        assert_eq!(scs.fold().into_spectrum(-1.0), expected);
    }

    #[test]
    fn test_fold_2x3x2() {
        let scs = Scs::from_range(0..12, [2, 3, 2]).unwrap();

        #[rustfmt::skip]
        let expected = Scs::new(
            [
                11., 11.,  
                11.,  5.5,
                5.5,  0.,
                
                11.,  5.5,
                 5.5, 0.,
                 0.,  0.,
            ],
            [2, 3, 2]
        ).unwrap();

        assert_eq!(scs.fold().into_spectrum(0.0), expected);
    }

    #[test]
    fn test_fold_3x3x3() {
        let scs = Scs::from_range(0..27, [3, 3, 3]).unwrap();

        #[rustfmt::skip]
        let expected = Scs::new(
        [
                26., 26., 26.,
                26., 26., 13.,
                26., 13.,  0.,
                
                26., 26., 13.,
                26., 13.,  0.,
                13.,  0.,  0.,

                26., 13.,  0.,
                13.,  0.,  0.,
                 0.,  0.,  0.,
            ],
        [3, 3, 3]
        ).unwrap();

        assert_eq!(scs.fold().into_spectrum(0.0), expected);
    }
}
