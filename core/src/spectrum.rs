//! Frequency and count spectra.

use std::{
    fmt,
    marker::PhantomData,
    ops::{AddAssign, Index, IndexMut, Range},
};

mod count;
pub use count::Count;

pub mod io;

pub mod iter;
use iter::FrequenciesIter;

mod folded;
pub use folded::Folded;

pub(crate) mod project;
use project::Projection;
pub use project::ProjectionError;

mod stat;
pub use stat::StatisticError;

use crate::array::{Array, Axis, Shape, ShapeError};

mod seal {
    #![deny(missing_docs)]
    pub trait Sealed {}
}
use seal::Sealed;

/// A type that can be used as marker for the state of a [`Spectrum`].
///
/// This trait is sealed and cannot be implemented outside this crate.
pub trait State: Sealed {
    #[doc(hidden)]
    fn debug_name() -> &'static str;
}

/// A marker struct for a [`Spectrum`] of frequencies.
///
/// See also [`Sfs`].
#[derive(Copy, Clone, Debug)]
pub struct Frequencies;
impl Sealed for Frequencies {}
impl State for Frequencies {
    fn debug_name() -> &'static str {
        "Sfs"
    }
}

/// A marker struct for a [`Spectrum`] of counts.
///
/// See also [`Scs`].
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Counts;
impl Sealed for Counts {}
impl State for Counts {
    fn debug_name() -> &'static str {
        "Scs"
    }
}

/// A site frequency spectrum.
pub type Sfs = Spectrum<Frequencies>;

/// A site count spectrum.
pub type Scs = Spectrum<Counts>;

/// A site spectrum.
///
/// The spectrum may either be over frequencies ([`Sfs`]) or counts ([`Scs`]).
#[derive(PartialEq)]
pub struct Spectrum<S: State> {
    array: Array<f64>,
    state: PhantomData<S>,
}

impl<S: State> Spectrum<S> {
    /// Returns the number of dimensions of the spectrum.
    pub fn dimensions(&self) -> usize {
        self.array.dimensions()
    }

    /// Returns the number of elements in the spectrum.
    pub fn elements(&self) -> usize {
        self.array.elements()
    }

    /// Returns a folded spectrum.
    pub fn fold(&self) -> Folded<S> {
        Folded::from_spectrum(self)
    }

    /// Returns the underlying array.
    pub fn inner(&self) -> &Array<f64> {
        &self.array
    }

    /// Returns a normalized frequency spectrum, consuming `self`.
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

    /// Returns an iterator over the allele frequencies of the elements in the spectrum in row-major
    /// order.
    ///
    /// Note that this is not an iterator over frequencies in the sense of a frequency spectrum, but
    /// in the sense of allele frequencies corresponding to indices in a spectrum.
    pub fn iter_frequencies(&self) -> FrequenciesIter<'_> {
        FrequenciesIter::new(self)
    }

    /// Returns the King statistic.
    ///
    /// See Manichaikul (2010) and Waples (2019) for details.
    ///
    /// # Errors
    ///
    /// If the spectrum is not a 3x3 2-dimensional spectrum.
    pub fn king(&self) -> Result<f64, StatisticError> {
        stat::King::from_spectrum(self)
            .map(|x| x.0)
            .map_err(Into::into)
    }

    /// Returns a spectrum with the provided axes marginalized out.
    ///
    /// # Errors
    ///
    /// If the provided axes contain duplicates, or if any of them are out of bounds.
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

    /// Normalizes the spectrum to frequencies in-place.
    ///
    /// See also [`Spectrum::into_normalized`] to normalize and convert to an [`Sfs`] at the
    /// type-level.
    pub fn normalize(&mut self) {
        let sum = self.sum();
        self.array.iter_mut().for_each(|x| *x /= sum);
    }

    /// Returns the average number of pairwise differences, also known as π.
    ///
    /// # Errors
    ///
    /// If the spectrum is not a 1-dimensional spectrum.
    pub fn pi(&self) -> Result<f64, StatisticError> {
        stat::Pi::from_spectrum(self)
            .map(|x| x.0)
            .map_err(Into::into)
    }

    /// Returns the average number of pairwise differences between two populations, also known as
    /// πₓᵧ or Dₓᵧ.
    ///
    /// See Nei and Li (1987).
    ///
    /// # Errors
    ///
    /// If the spectrum is not a 1-dimensional spectrum.
    pub fn pi_xy(&self) -> Result<f64, StatisticError> {
        stat::PiXY::from_spectrum(self)
            .map(|x| x.0)
            .map_err(Into::into)
    }

    /// Returns a spectrum projected down to a shape.
    ///
    /// The projection is based on hypergeometric down-sampling. See Marth (2004) and Gutenkunst
    /// (2009) for details. Note that projecting a spectrum after creation may cause problems;
    /// prefer projecting site-wise during creation where possible.
    ///
    /// # Errors
    ///
    /// Errors if the projected shape is not valid for the provided spectrum.
    pub fn project<T>(&self, project_to: T) -> Result<Self, ProjectionError>
    where
        T: Into<Shape>,
    {
        let project_to = project_to.into();
        let mut projection = Projection::from_shapes(self.shape().clone(), project_to.clone())?;
        let mut new = Scs::from_zeros(project_to);

        for (&weight, from) in self.array.iter().zip(self.array.iter_indices().map(Count)) {
            projection
                .project_unchecked(&from)
                .into_weighted(weight)
                .add_unchecked(&mut new);
        }

        Ok(new.into_state_unchecked())
    }

    /// Returns the R0 statistic.
    ///
    /// See Waples (2019) for details.
    ///
    /// # Errors
    ///
    /// If the spectrum is not a 3x3 2-dimensional spectrum.
    pub fn r0(&self) -> Result<f64, StatisticError> {
        stat::R0::from_spectrum(self)
            .map(|x| x.0)
            .map_err(Into::into)
    }

    /// Returns the R0 statistic.
    ///
    /// See Waples (2019) for details.
    ///
    /// # Errors
    ///
    /// If the spectrum is not a 3x3 2-dimensional spectrum.
    pub fn r1(&self) -> Result<f64, StatisticError> {
        stat::R1::from_spectrum(self)
            .map(|x| x.0)
            .map_err(Into::into)
    }

    /// Returns the shape of the spectrum.
    pub fn shape(&self) -> &Shape {
        self.array.shape()
    }

    /// Returns the sum of elements in the spectrum.
    pub fn sum(&self) -> f64 {
        self.array.iter().sum::<f64>()
    }

    /// Returns Watterson's estimator of the mutation-scaled effective population size θ.
    ///
    /// # Errors
    ///
    /// If the spectrum is not a 1-dimensional spectrum.
    pub fn theta_watterson(&self) -> Result<f64, StatisticError> {
        stat::Theta::<stat::theta::Watterson>::from_spectrum(self)
            .map(|x| x.0)
            .map_err(Into::into)
    }
}

impl Scs {
    /// Returns Fu and Li's D difference statistic.
    ///
    /// See Fu and Li (1993).
    ///
    /// # Errors
    ///
    /// If the spectrum is not a 1-dimensional spectrum.
    pub fn d_fu_li(&self) -> Result<f64, StatisticError> {
        stat::D::<stat::d::FuLi>::from_scs(self)
            .map(|x| x.0)
            .map_err(Into::into)
    }

    /// Returns Tajima's D difference statistic.
    ///
    /// See Tajima (1989).
    ///
    /// # Errors
    ///
    /// If the spectrum is not a 1-dimensional spectrum.
    pub fn d_tajima(&self) -> Result<f64, StatisticError> {
        stat::D::<stat::d::Tajima>::from_scs(self)
            .map(|x| x.0)
            .map_err(Into::into)
    }

    /// Creates a new spectrum from a range and a shape.
    ///
    /// This is mainly intended for testing and illustration.
    ///
    /// # Errors
    ///
    /// If the number of items in the range does not match the provided shape.
    pub fn from_range<S>(range: Range<usize>, shape: S) -> Result<Self, ShapeError>
    where
        S: Into<Shape>,
    {
        Array::from_iter(range.map(|v| v as f64), shape).map(Self::from)
    }

    /// Creates a new one-dimensional spectrum from a vector.
    pub fn from_vec<T>(vec: T) -> Self
    where
        T: Into<Vec<f64>>,
    {
        let vec = vec.into();
        let shape = vec.len();
        Self::new(vec, shape).unwrap()
    }

    /// Creates a new spectrum filled with zeros to a shape.
    pub fn from_zeros<S>(shape: S) -> Self
    where
        S: Into<Shape>,
    {
        Self::from(Array::from_zeros(shape))
    }

    /// Returns a mutable reference to the underlying array.
    pub fn inner_mut(&mut self) -> &mut Array<f64> {
        &mut self.array
    }

    /// Creates a new spectrum from data in row-major order and a shape.
    ///
    /// # Errors
    ///
    /// If the number of items in the data does not match the provided shape.
    pub fn new<D, S>(data: D, shape: S) -> Result<Self, ShapeError>
    where
        D: Into<Vec<f64>>,
        S: Into<Shape>,
    {
        Array::new(data, shape).map(Self::from)
    }

    /// Returns the number of sites segregating in any population in the spectrum.
    pub fn segregating_sites(&self) -> f64 {
        let n = self.elements();

        self.array.iter().take(n - 1).skip(1).sum()
    }
}

impl Sfs {
    /// Returns the f₂ statistic.
    ///
    /// See Reich (2009) and Peter (2016) for details.
    ///
    /// # Errors
    ///
    /// If the spectrum is not a 2-dimensional spectrum.
    pub fn f2(&self) -> Result<f64, StatisticError> {
        stat::F2::from_sfs(self).map(|x| x.0).map_err(Into::into)
    }

    /// Returns the f₃(A; B, C)-statistic, where A, B, C is in the order of the populations in the
    /// spectrum.
    ///
    /// Note that f₃ may also be calculated as a linear combination of f₂, which is often going to
    /// be more efficient and flexible.
    ///
    /// See Reich (2009) and Peter (2016) for details.
    ///
    /// # Errors
    ///
    /// If the spectrum is not a 3-dimensional spectrum.
    pub fn f3(&self) -> Result<f64, StatisticError> {
        stat::F3::from_sfs(self).map(|x| x.0).map_err(Into::into)
    }

    /// Returns the f₄(A, B; C, D)-statistic, where A, B, C is in the order of the populations in
    /// the spectrum.
    ///
    /// Note that f₄ may also be calculated as a linear combination of f₂, which is often going to
    /// be more efficient and flexible.
    ///
    /// See Reich (2009) and Peter (2016) for details.
    ///
    /// # Errors
    ///
    /// If the spectrum is not a 4-dimensional spectrum.
    pub fn f4(&self) -> Result<f64, StatisticError> {
        stat::F4::from_sfs(self).map(|x| x.0).map_err(Into::into)
    }

    /// Returns Hudson's estimator of Fst.
    ///
    /// See Bhatia (2013) for details. (This uses a "ratio of estimates" as recommended there.)
    ///
    /// # Errors
    ///
    /// If the spectrum is not a 2-dimensional spectrum.
    pub fn fst(&self) -> Result<f64, StatisticError> {
        stat::Fst::from_sfs(self).map(|x| x.0).map_err(Into::into)
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

impl<S: State> fmt::Debug for Spectrum<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct(S::debug_name())
            .field("array", &self.array)
            .finish()
    }
}

impl AddAssign<&Count> for Scs {
    fn add_assign(&mut self, count: &Count) {
        self[count] += 1.0;
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

/// An error associated with marginalizing a spectrum.
#[derive(Debug, Eq, PartialEq)]
pub enum MarginalizationError {
    /// An axis is duplicated.
    DuplicateAxis {
        /// The index of the duplicated axis.
        axis: usize,
    },
    /// An axis is out of bounds.
    AxisOutOfBounds {
        /// The axis that is out of bounds.
        axis: usize,
        /// The number of dimensions in the spectrum.
        dimensions: usize,
    },
    /// Too many axes provided.
    TooManyAxes {
        /// The number of provided axes.
        axes: usize,
        /// The number of dimensions in the spectrum.
        dimensions: usize,
    },
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

    use crate::approx::ApproxEq;

    impl<S: State> ApproxEq for Spectrum<S> {
        const DEFAULT_EPSILON: Self::Epsilon = <f64 as ApproxEq>::DEFAULT_EPSILON;

        type Epsilon = <f64 as ApproxEq>::Epsilon;

        fn approx_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
            self.array.approx_eq(&other.array, epsilon)
        }
    }

    #[test]
    fn test_marginalize_axis_2d() {
        let scs = Scs::from_range(0..9, [3, 3]).unwrap();

        assert_eq!(
            scs.marginalize_axis(Axis(0)),
            Scs::new([9., 12., 15.], 3).unwrap()
        );

        assert_eq!(
            scs.marginalize_axis(Axis(1)),
            Scs::new([3., 12., 21.], 3).unwrap()
        );
    }

    #[test]
    fn test_marginalize_axis_3d() {
        let scs = Scs::from_range(0..27, [3, 3, 3]).unwrap();

        assert_eq!(
            scs.marginalize_axis(Axis(0)),
            Scs::new([27., 30., 33., 36., 39., 42., 45., 48., 51.], [3, 3]).unwrap()
        );

        assert_eq!(
            scs.marginalize_axis(Axis(1)),
            Scs::new([9., 12., 15., 36., 39., 42., 63., 66., 69.], [3, 3]).unwrap()
        );

        assert_eq!(
            scs.marginalize_axis(Axis(2)),
            Scs::new([3., 12., 21., 30., 39., 48., 57., 66., 75.], [3, 3]).unwrap()
        );
    }

    #[test]
    fn test_marginalize_3d() {
        let scs = Scs::from_range(0..27, [3, 3, 3]).unwrap();

        let expected = Scs::new([90., 117., 144.], [3]).unwrap();
        assert_eq!(scs.marginalize(&[Axis(0), Axis(2)]).unwrap(), expected);
        assert_eq!(scs.marginalize(&[Axis(2), Axis(0)]).unwrap(), expected);
    }

    #[test]
    fn test_marginalize_too_many_axes() {
        let scs = Scs::from_range(0..9, [3, 3]).unwrap();

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
        let scs = Scs::from_range(0..27, [3, 3, 3]).unwrap();

        assert_eq!(
            scs.marginalize(&[Axis(1), Axis(1)]),
            Err(MarginalizationError::DuplicateAxis { axis: 1 }),
        );
    }

    #[test]
    fn test_marginalize_axis_out_ouf_bounds() {
        let scs = Scs::from_range(0..9, [3, 3]).unwrap();

        assert_eq!(
            scs.marginalize(&[Axis(2)]),
            Err(MarginalizationError::AxisOutOfBounds {
                axis: 2,
                dimensions: 2
            }),
        );
    }

    #[test]
    fn test_project_7_to_3() {
        let scs = Scs::from_range(0..7, 7).unwrap();
        let projected = scs.project(3).unwrap();
        let expected = Scs::new([2.333333, 7.0, 11.666667], 3).unwrap();
        assert_approx_eq!(projected, expected, epsilon = 1e-6);
    }

    #[test]
    fn test_project_7_to_7_is_identity() {
        let scs = Scs::from_range(0..7, 7).unwrap();
        let projected = scs.project(7).unwrap();
        assert_eq!(scs, projected);
    }

    #[test]
    fn test_project_7_to_8_is_error() {
        let scs = Scs::from_range(0..7, 7).unwrap();
        let result = scs.project(8);

        assert!(matches!(
            result,
            Err(ProjectionError::InvalidProjection { .. })
        ));
    }

    #[test]
    fn test_project_7_to_0_is_error() {
        let scs = Scs::from_range(0..7, 7).unwrap();
        let result = scs.project(0);

        assert!(matches!(result, Err(ProjectionError::Zero)));
    }

    #[test]
    fn test_project_3x3_to_2x2() {
        let scs = Scs::from_range(0..9, [3, 3]).unwrap();
        let projected = scs.project([2, 2]).unwrap();
        let expected = Scs::new([3.0, 6.0, 12.0, 15.0], [2, 2]).unwrap();
        assert_approx_eq!(projected, expected, epsilon = 1e-6);
    }
}
