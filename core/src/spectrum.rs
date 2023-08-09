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

pub mod project;
use project::{Projection, ProjectionError};

pub mod stat;

use crate::array::{Array, Axis, Shape, ShapeError};

mod seal {
    #![deny(missing_docs)]
    pub trait Sealed {}
}
use seal::Sealed;
pub trait State: Sealed {
    #[doc(hidden)]
    fn debug_name() -> &'static str;
}

#[derive(Copy, Clone, Debug)]
pub struct Frequencies;
impl Sealed for Frequencies {}
impl State for Frequencies {
    fn debug_name() -> &'static str {
        "Sfs"
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Counts;
impl Sealed for Counts {}
impl State for Counts {
    fn debug_name() -> &'static str {
        "Scs"
    }
}

pub type Sfs = Spectrum<Frequencies>;
pub type Scs = Spectrum<Counts>;

#[derive(PartialEq)]
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

    pub fn fold(&self) -> Folded<S> {
        Folded::from_spectrum(self)
    }

    pub fn inner(&self) -> &Array<f64> {
        &self.array
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

    pub fn project<T>(&self, to: T) -> Result<Self, ProjectionError>
    where
        T: Into<Shape>,
    {
        let to = to.into();
        let projection = Projection::new(
            Count::from_shape(self.shape().clone()),
            Count::from_shape(to.clone()),
        )?;
        let mut new = Scs::from_zeros(to);

        for (&weight, from) in self.array.iter().zip(self.array.iter_indices().map(Count)) {
            projection
                .project_unchecked(&from)
                .into_weighted(weight)
                .add_unchecked(&mut new);
        }

        Ok(new.into_state_unchecked())
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
    pub fn inner_mut(&mut self) -> &mut Array<f64> {
        &mut self.array
    }

    pub fn new<D, S>(data: D, shape: S) -> Result<Self, ShapeError>
    where
        Vec<f64>: From<D>,
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
    fn test_project_3x3_to_2x2() {
        let scs = Scs::from_range(0..9, [3, 3]).unwrap();
        let projected = scs.project([2, 2]).unwrap();
        let expected = Scs::new([3.0, 6.0, 12.0, 15.0], [2, 2]).unwrap();
        assert_approx_eq!(projected, expected, epsilon = 1e-6);
    }
}
