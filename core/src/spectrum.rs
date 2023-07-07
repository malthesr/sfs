use std::{
    cmp::Ordering,
    fmt,
    marker::PhantomData,
    ops::{Index, IndexMut, Range},
};

pub mod io;

pub mod iter;
use iter::FrequenciesIter;

pub mod stat;

use crate::array::{Array, Axis, Shape, ShapeError};

mod seal {
    #![deny(missing_docs)]
    pub trait Sealed {}
}
use seal::Sealed;
pub trait State: Sealed {}

#[derive(Copy, Clone, Debug)]
pub struct Frequencies;
impl Sealed for Frequencies {}
impl State for Frequencies {}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Counts;
impl Sealed for Counts {}
impl State for Counts {}

pub type Sfs = Spectrum<Frequencies>;
pub type Scs = Spectrum<Counts>;

#[derive(Debug, PartialEq)]
pub struct Spectrum<S: State> {
    array: Array<f64>,
    state: PhantomData<S>,
}

impl<S: State> Spectrum<S> {
    pub fn dimensions(&self) -> usize {
        self.array.dimensions()
    }

    pub fn elements(&self) -> usize {
        self.array.elements()
    }

    pub fn into_normalized(mut self) -> Sfs {
        self.normalize();
        self.into_state_unchecked()
    }

    fn into_state_unchecked<R: State>(self) -> Spectrum<R> {
        Spectrum {
            array: self.array,
            state: PhantomData,
        }
    }

    pub fn iter_frequencies(&self) -> FrequenciesIter<'_> {
        FrequenciesIter::new(self)
    }

    pub fn marginalize(&self, axes: &[Axis]) -> Result<Self, MarginalizationError> {
        if let Some(duplicate) = axes.iter().enumerate().find_map(|(i, axis)| {
            axes.get(i + 1..)
                .and_then(|slice| slice.contains(axis).then_some(axis))
        }) {
            return Err(MarginalizationError::DuplicateAxis { axis: duplicate.0 });
        };

        if let Some(out_of_bounds) = axes.iter().find(|axis| axis.0 >= self.dimensions()) {
            return Err(MarginalizationError::AxisOutOfBounds {
                axis: out_of_bounds.0,
                dimensions: self.dimensions(),
            });
        };

        if axes.len() >= self.dimensions() {
            return Err(MarginalizationError::TooManyAxes {
                axes: axes.len(),
                dimensions: self.dimensions(),
            });
        }

        let is_sorted = axes.windows(2).all(|w| w[0] <= w[1]);
        if is_sorted {
            Ok(self.marginalize_unchecked(axes))
        } else {
            let mut axes = axes.to_vec();
            axes.sort();
            Ok(self.marginalize_unchecked(&axes))
        }
    }

    fn marginalize_axis(&self, axis: Axis) -> Self {
        Scs::from(self.array.sum(axis)).into_state_unchecked()
    }

    fn marginalize_unchecked(&self, axes: &[Axis]) -> Self {
        let mut spectrum = self.clone();

        // As we marginalize out axes one by one, the axes shift down,
        // so we subtract the number already removed and rely on axes having been sorted
        axes.iter()
            .enumerate()
            .map(|(removed, original)| Axis(original.0 - removed))
            .for_each(|axis| {
                spectrum = spectrum.marginalize_axis(axis);
            });

        spectrum
    }

    pub fn normalize(&mut self) {
        let sum = self.sum();
        self.array.iter_mut().for_each(|x| *x /= sum);
    }

    pub fn shape(&self) -> &Shape {
        self.array.shape()
    }

    pub fn sum(&self) -> f64 {
        self.array.iter().sum::<f64>()
    }
}

impl Scs {
    pub fn new<S>(data: Vec<f64>, shape: S) -> Result<Self, ShapeError>
    where
        Shape: From<S>,
    {
        Array::new(data, shape).map(Self::from)
    }

    pub fn from_range<S>(range: Range<usize>, shape: S) -> Result<Self, ShapeError>
    where
        Shape: From<S>,
    {
        Array::from_iter(range.map(|v| v as f64), shape).map(Self::from)
    }

    pub fn from_zeros<S>(shape: S) -> Self
    where
        Shape: From<S>,
    {
        Self::from(Array::from_zeros(shape))
    }

    pub fn fold(&self, sentry: f64) -> Self {
        let n = self.elements();
        let total_count = self.shape().iter().sum::<usize>() - self.shape().len();

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
        let mut folded = Self::from_zeros(self.shape().clone());

        // We iterate over indices rather than values since we have to mutate on the array
        // while looking at it from both directions.
        (0..n).zip((0..n).rev()).for_each(|(i, rev_i)| {
            let count = self.shape().index_sum_from_flat_unchecked(i);

            let src = self.array.as_slice();
            let dst = folded.array.as_mut_slice();

            match (count.cmp(&mid_count), has_diagonal) {
                (Ordering::Less, _) | (Ordering::Equal, false) => {
                    // We are in the upper part of the spectrum that should be folded onto.
                    dst[i] = src[i] + src[rev_i];
                }
                (Ordering::Equal, true) => {
                    // We are on a diagonal, which must be handled as a special case:
                    // there are apparently different opinions on what the most correct
                    // thing to do is. This adopts the same strategy as e.g. in dadi.
                    dst[i] = 0.5 * src[i] + 0.5 * src[rev_i];
                }
                (Ordering::Greater, _) => {
                    // We are in the lower part of the spectrum to be filled with sentry values.
                    dst[i] = sentry;
                }
            }
        });

        folded
    }
}

impl<S: State> Clone for Spectrum<S> {
    fn clone(&self) -> Self {
        Self {
            array: self.array.clone(),
            state: PhantomData,
        }
    }
}

impl From<Array<f64>> for Scs {
    fn from(array: Array<f64>) -> Self {
        Self {
            array,
            state: PhantomData,
        }
    }
}

impl<I, S: State> Index<I> for Spectrum<S>
where
    I: AsRef<[usize]>,
{
    type Output = f64;

    fn index(&self, index: I) -> &Self::Output {
        self.array.index(index)
    }
}

impl<I, S: State> IndexMut<I> for Spectrum<S>
where
    I: AsRef<[usize]>,
{
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        self.array.index_mut(index)
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum MarginalizationError {
    DuplicateAxis { axis: usize },
    AxisOutOfBounds { axis: usize, dimensions: usize },
    TooManyAxes { axes: usize, dimensions: usize },
}

impl fmt::Display for MarginalizationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MarginalizationError::DuplicateAxis { axis } => {
                write!(f, "cannot marginalize with duplicate axis {axis}")
            }
            MarginalizationError::AxisOutOfBounds { axis, dimensions } => write!(
                f,
                "cannot marginalize axis {axis} in spectrum with {dimensions} dimensions"
            ),
            MarginalizationError::TooManyAxes { axes, dimensions } => write!(
                f,
                "cannot marginalize a total of {axes} axes in spectrum with {dimensions} dimensions"
            ),
        }
    }
}

impl std::error::Error for MarginalizationError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fold_4() {
        let scs = Scs::from_range(0..4, vec![4]).unwrap();
        let expected = Scs::new(vec![3., 3., 0., 0.], vec![4]).unwrap();
        assert_eq!(scs.fold(0.), expected);
    }

    #[test]
    fn test_fold_5() {
        let scs = Scs::from_range(0..5, vec![5]).unwrap();
        let expected = Scs::new(vec![4., 4., 2., -1., -1.], vec![5]).unwrap();
        assert_eq!(scs.fold(-1.), expected);
    }

    #[test]
    fn test_fold_3x3() {
        let scs = Scs::from_range(0..9, vec![3, 3]).unwrap();

        #[rustfmt::skip]
        let expected = Scs::new(
            vec![
                8., 8., 4.,
                8., 4., 0.,
                4., 0., 0.,
            ],
            vec![3, 3]
        ).unwrap();

        assert_eq!(scs.fold(0.), expected);
    }

    #[test]
    fn test_fold_2x4() {
        let scs = Scs::from_range(0..8, vec![2, 4]).unwrap();

        #[rustfmt::skip]
        let expected = Scs::new(
            vec![
                7., 7.,            3.5, f64::INFINITY,
                7., 3.5, f64::INFINITY, f64::INFINITY,
            ],
            vec![2, 4]
        ).unwrap();

        assert_eq!(scs.fold(f64::INFINITY), expected);
    }

    #[test]
    fn test_fold_3x4() {
        let scs = Scs::from_range(0..12, vec![3, 4]).unwrap();

        #[rustfmt::skip]
        let expected = Scs::new(
            vec![
                11., 11., 11., 0.,
                11., 11.,  0., 0.,
                11.,  0.,  0., 0.,
            ],
            vec![3, 4]
        ).unwrap();

        assert_eq!(scs.fold(0.), expected);
    }

    #[test]
    fn test_fold_3x7() {
        let scs = Scs::from_range(0..21, vec![3, 7]).unwrap();

        #[rustfmt::skip]
        let expected = Scs::new(
            vec![
                20., 20., 20., 20., 10., 0., 0.,
                20., 20., 20., 10.,  0., 0., 0.,
                20., 20., 10.,  0.,  0., 0., 0.,
            ],
            vec![3, 7]
        ).unwrap();

        assert_eq!(scs.fold(0.), expected);
    }

    #[test]
    fn test_fold_2x2x2() {
        let scs = Scs::from_range(0..8, vec![2, 2, 2]).unwrap();

        #[rustfmt::skip]
        let expected = Scs::new(
            vec![
                 7.,  7.,
                 7., -1.,
                
                 7., -1.,
                -1., -1.,
            ],
            vec![2, 2, 2]
        ).unwrap();

        assert_eq!(scs.fold(-1.), expected);
    }

    #[test]
    fn test_fold_2x3x2() {
        let scs = Scs::from_range(0..12, vec![2, 3, 2]).unwrap();

        #[rustfmt::skip]
        let expected = Scs::new(
            vec![
                11., 11.,  
                11.,  5.5,
                5.5,  0.,
                
                11.,  5.5,
                 5.5, 0.,
                 0.,  0.,
            ],
            vec![2, 3, 2]
        ).unwrap();

        assert_eq!(scs.fold(0.), expected);
    }

    #[test]
    fn test_fold_3x3x3() {
        let scs = Scs::from_range(0..27, vec![3, 3, 3]).unwrap();

        #[rustfmt::skip]
        let expected = Scs::new(
            vec![
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
            vec![3, 3, 3]
        ).unwrap();

        assert_eq!(scs.fold(0.), expected);
    }

    #[test]
    fn test_marginalize_axis_2d() {
        let scs = Scs::from_range(0..9, vec![3, 3]).unwrap();

        assert_eq!(
            scs.marginalize_axis(Axis(0)),
            Scs::new(vec![9., 12., 15.], vec![3]).unwrap()
        );

        assert_eq!(
            scs.marginalize_axis(Axis(1)),
            Scs::new(vec![3., 12., 21.], vec![3]).unwrap()
        );
    }

    #[test]
    fn test_marginalize_axis_3d() {
        let scs = Scs::from_range(0..27, vec![3, 3, 3]).unwrap();

        assert_eq!(
            scs.marginalize_axis(Axis(0)),
            Scs::new(
                vec![27., 30., 33., 36., 39., 42., 45., 48., 51.],
                vec![3, 3]
            )
            .unwrap()
        );

        assert_eq!(
            scs.marginalize_axis(Axis(1)),
            Scs::new(vec![9., 12., 15., 36., 39., 42., 63., 66., 69.], vec![3, 3]).unwrap()
        );

        assert_eq!(
            scs.marginalize_axis(Axis(2)),
            Scs::new(vec![3., 12., 21., 30., 39., 48., 57., 66., 75.], vec![3, 3]).unwrap()
        );
    }

    #[test]
    fn test_marginalize_3d() {
        let scs = Scs::from_range(0..27, vec![3, 3, 3]).unwrap();

        let expected = Scs::new(vec![90., 117., 144.], vec![3]).unwrap();
        assert_eq!(scs.marginalize(&[Axis(0), Axis(2)]).unwrap(), expected);
        assert_eq!(scs.marginalize(&[Axis(2), Axis(0)]).unwrap(), expected);
    }

    #[test]
    fn test_marginalize_too_many_axes() {
        let scs = Scs::from_range(0..9, vec![3, 3]).unwrap();

        assert_eq!(
            scs.marginalize(&[Axis(0), Axis(1)]),
            Err(MarginalizationError::TooManyAxes {
                axes: 2,
                dimensions: 2
            }),
        );
    }

    #[test]
    fn test_marginalize_duplicate_axis() {
        let scs = Scs::from_range(0..27, vec![3, 3, 3]).unwrap();

        assert_eq!(
            scs.marginalize(&[Axis(1), Axis(1)]),
            Err(MarginalizationError::DuplicateAxis { axis: 1 }),
        );
    }

    #[test]
    fn test_marginalize_axis_out_ouf_bounds() {
        let scs = Scs::from_range(0..9, vec![3, 3]).unwrap();

        assert_eq!(
            scs.marginalize(&[Axis(2)]),
            Err(MarginalizationError::AxisOutOfBounds {
                axis: 2,
                dimensions: 2
            }),
        );
    }
}
