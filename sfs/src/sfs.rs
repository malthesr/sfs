use std::{
    cmp::Ordering,
    fmt,
    ops::{Index, IndexMut, Range},
};

pub mod io;

pub mod iter;
use iter::{AxisIter, FrequenciesIter, IndicesIter};

mod shape;
use shape::Strides;
pub use shape::{Axis, Shape};

pub mod stat;

mod view;
pub use view::View;

pub type NormSfs = Sfs<true>;

#[derive(Clone, Debug, PartialEq)]
pub struct Sfs<const N: bool = false> {
    pub(crate) data: Vec<f64>,
    pub(crate) shape: Shape,
    pub(crate) strides: Strides,
}

impl<const N: bool> Sfs<N> {
    pub fn as_slice(&self) -> &[f64] {
        self.data.as_slice()
    }

    pub fn dimensions(&self) -> usize {
        self.shape.len()
    }

    pub fn get_axis(&self, axis: Axis, index: usize) -> Option<View<'_>> {
        if axis.0 > self.dimensions() || index >= self.shape[axis.0] {
            None
        } else {
            let offset = index * self.strides[axis.0];
            let data = &self.data;
            let shape = self.shape.remove_axis(axis);
            let strides = self.strides.remove_axis(axis);

            Some(View::new_unchecked(data, offset, shape, strides))
        }
    }

    pub fn into_normalized(mut self) -> NormSfs {
        self.normalize();
        self.with_normalization()
    }

    pub fn iter(&self) -> std::slice::Iter<'_, f64> {
        self.data.iter()
    }

    pub fn iter_axis(&self, axis: Axis) -> AxisIter<'_, N> {
        AxisIter::new(self, axis)
    }

    pub fn iter_frequencies(&self) -> FrequenciesIter<'_, N> {
        FrequenciesIter::new(self)
    }

    pub fn iter_indices(&self) -> IndicesIter<'_, N> {
        IndicesIter::new(self)
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

    fn marginalize_unchecked(&self, axes: &[Axis]) -> Self {
        let mut sfs = self.clone();

        // As we marginalize out axes one by one, the axes shift down,
        // so we subtract the number already removed and rely on axes having been sorted
        axes.iter()
            .enumerate()
            .map(|(removed, original)| Axis(original.0 - removed))
            .for_each(|axis| {
                sfs = sfs.marginalize_axis(axis);
            });

        sfs
    }

    fn marginalize_axis(&self, axis: Axis) -> Self {
        let new_shape = Shape(self.shape.remove_axis(axis).iter().copied().collect());

        self.iter_axis(axis)
            .fold(Sfs::from_zeros(new_shape), |mut sfs, view| {
                sfs.iter_mut().zip(view.iter()).for_each(|(x, y)| *x += y);
                sfs
            })
            .with_normalization()
    }

    pub fn new_unchecked(data: Vec<f64>, shape: Shape) -> Self {
        Self {
            data,
            strides: shape.strides(),
            shape,
        }
    }

    pub fn normalize(&mut self) {
        let sum = self.data.iter().sum::<f64>();

        self.data.iter_mut().for_each(|x| *x /= sum);
    }

    pub fn shape(&self) -> &Shape {
        &self.shape
    }

    fn with_normalization<const M: bool>(self) -> Sfs<M> {
        Sfs {
            data: self.data,
            shape: self.shape,
            strides: self.strides,
        }
    }
}

impl Sfs {
    pub fn fold(&self, sentry: f64) -> Self {
        let n = self.data.len();
        let total_count = self.shape.iter().sum::<usize>() - self.shape.len();

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
        let mut folded = Self::from_zeros(self.shape.clone());

        // We iterate over indices rather than values since we have to mutate on the array
        // while looking at it from both directions.
        (0..n).zip((0..n).rev()).for_each(|(i, rev_i)| {
            let count = self.shape.index_sum_from_flat_unchecked(i);

            match (count.cmp(&mid_count), has_diagonal) {
                (Ordering::Less, _) | (Ordering::Equal, false) => {
                    // We are in the upper part of the spectrum that should be folded onto.
                    folded.data[i] = self.data[i] + self.data[rev_i];
                }
                (Ordering::Equal, true) => {
                    // We are on a diagonal, which must be handled as a special case:
                    // there are apparently different opinions on what the most correct
                    // thing to do is. This adopts the same strategy as e.g. in dadi.
                    folded.data[i] = 0.5 * self.data[i] + 0.5 * self.data[rev_i];
                }
                (Ordering::Greater, _) => {
                    // We are in the lower part of the spectrum to be filled with sentry values.
                    folded.data[i] = sentry;
                }
            }
        });

        folded
    }

    pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, f64> {
        self.data.iter_mut()
    }

    pub fn new(data: Vec<f64>, shape: Shape) -> Result<Self, ShapeError> {
        if data.len() == shape.iter().product() {
            Ok(Self::new_unchecked(data, shape))
        } else {
            Err(ShapeError {
                shape,
                n: data.len(),
            })
        }
    }

    pub fn from_iter<I>(iter: I, shape: Shape) -> Result<Self, ShapeError>
    where
        I: IntoIterator<Item = f64>,
    {
        let data = iter.into_iter().collect();

        Self::new(data, shape)
    }

    pub fn from_range(range: Range<usize>, shape: Shape) -> Result<Self, ShapeError> {
        Self::from_iter(range.map(|v| v as f64), shape)
    }

    pub fn from_zeros(shape: Shape) -> Self {
        Self::new_unchecked(vec![0.0; shape.iter().product()], shape)
    }
}

impl<I, const N: bool> Index<I> for Sfs<N>
where
    I: AsRef<[usize]>,
{
    type Output = f64;

    fn index(&self, index: I) -> &Self::Output {
        self.strides
            .flat_index(&self.shape, index)
            .and_then(|flat| self.data.get(flat))
            .expect("index out of bounds")
    }
}

impl<I, const N: bool> IndexMut<I> for Sfs<N>
where
    I: AsRef<[usize]>,
{
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        self.strides
            .flat_index(&self.shape, index)
            .and_then(|flat| self.data.get_mut(flat))
            .expect("index out of bounds")
    }
}

#[derive(Debug)]
pub struct ShapeError {
    shape: Shape,
    n: usize,
}

impl fmt::Display for ShapeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ShapeError { shape, n } = self;
        write!(
            f,
            "cannot construct SFS with shape {shape} from {n} elements"
        )
    }
}

impl std::error::Error for ShapeError {}

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
                "cannot marginalize axis {axis} in SFS with {dimensions} dimensions"
            ),
            MarginalizationError::TooManyAxes { axes, dimensions } => write!(
                f,
                "cannot marginalize a total of {axes} axes in SFS with {dimensions} dimensions"
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
        let sfs = Sfs::from_range(0..4, Shape(vec![4])).unwrap();
        let expected = Sfs::new(vec![3., 3., 0., 0.], Shape(vec![4])).unwrap();

        assert_eq!(sfs.fold(0.), expected);
    }

    #[test]
    fn test_fold_5() {
        let sfs = Sfs::from_range(0..5, Shape(vec![5])).unwrap();

        let expected = Sfs::new(vec![4., 4., 2., -1., -1.], Shape(vec![5])).unwrap();

        assert_eq!(sfs.fold(-1.), expected);
    }

    #[test]
    fn test_fold_3x3() {
        let sfs = Sfs::from_range(0..9, Shape(vec![3, 3])).unwrap();

        #[rustfmt::skip]
        let expected = Sfs::new(
            vec![
                8., 8., 4.,
                8., 4., 0.,
                4., 0., 0.,
            ],
            Shape(vec![3, 3])
        ).unwrap();

        assert_eq!(sfs.fold(0.), expected);
    }

    #[test]
    fn test_fold_2x4() {
        let sfs = Sfs::from_range(0..8, Shape(vec![2, 4])).unwrap();

        #[rustfmt::skip]
        let expected = Sfs::new(
            vec![
                7., 7.,            3.5, f64::INFINITY,
                7., 3.5, f64::INFINITY, f64::INFINITY,
            ],
            Shape(vec![2, 4])
        ).unwrap();

        assert_eq!(sfs.fold(f64::INFINITY), expected);
    }

    #[test]
    fn test_fold_3x4() {
        let sfs = Sfs::from_range(0..12, Shape(vec![3, 4])).unwrap();

        #[rustfmt::skip]
        let expected = Sfs::new(
            vec![
                11., 11., 11., 0.,
                11., 11.,  0., 0.,
                11.,  0.,  0., 0.,
            ],
            Shape(vec![3, 4])
        ).unwrap();

        assert_eq!(sfs.fold(0.), expected);
    }

    #[test]
    fn test_fold_3x7() {
        let sfs = Sfs::from_range(0..21, Shape(vec![3, 7])).unwrap();

        #[rustfmt::skip]
        let expected = Sfs::new(
            vec![
                20., 20., 20., 20., 10., 0., 0.,
                20., 20., 20., 10.,  0., 0., 0.,
                20., 20., 10.,  0.,  0., 0., 0.,
            ],
            Shape(vec![3, 7])
        ).unwrap();

        assert_eq!(sfs.fold(0.), expected);
    }

    #[test]
    fn test_fold_2x2x2() {
        let sfs = Sfs::from_range(0..8, Shape(vec![2, 2, 2])).unwrap();

        #[rustfmt::skip]
        let expected = Sfs::new(
            vec![
                 7.,  7.,
                 7., -1.,
                
                 7., -1.,
                -1., -1.,
            ],
            Shape(vec![2, 2, 2])
        ).unwrap();

        assert_eq!(sfs.fold(-1.), expected);
    }

    #[test]
    fn test_fold_2x3x2() {
        let sfs = Sfs::from_range(0..12, Shape(vec![2, 3, 2])).unwrap();

        #[rustfmt::skip]
        let expected = Sfs::new(
            vec![
                11., 11.,  
                11.,  5.5,
                5.5,  0.,
                
                11.,  5.5,
                 5.5, 0.,
                 0.,  0.,
            ],
            Shape(vec![2, 3, 2])
        ).unwrap();

        assert_eq!(sfs.fold(0.), expected);
    }

    #[test]
    fn test_fold_3x3x3() {
        let sfs = Sfs::from_range(0..27, Shape(vec![3, 3, 3])).unwrap();

        #[rustfmt::skip]
        let expected = Sfs::new(
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
            Shape(vec![3, 3, 3])
        ).unwrap();

        assert_eq!(sfs.fold(0.), expected);
    }

    #[test]
    fn test_marginalize_axis_2d() {
        let sfs = Sfs::from_range(0..9, Shape(vec![3, 3])).unwrap();

        assert_eq!(
            sfs.marginalize_axis(Axis(0)),
            Sfs::new(vec![9., 12., 15.], Shape(vec![3])).unwrap()
        );

        assert_eq!(
            sfs.marginalize_axis(Axis(1)),
            Sfs::new(vec![3., 12., 21.], Shape(vec![3])).unwrap()
        );
    }

    #[test]
    fn test_marginalize_axis_3d() {
        let sfs = Sfs::from_range(0..27, Shape(vec![3, 3, 3])).unwrap();

        assert_eq!(
            sfs.marginalize_axis(Axis(0)),
            Sfs::new(
                vec![27., 30., 33., 36., 39., 42., 45., 48., 51.],
                Shape(vec![3, 3])
            )
            .unwrap()
        );

        assert_eq!(
            sfs.marginalize_axis(Axis(1)),
            Sfs::new(
                vec![9., 12., 15., 36., 39., 42., 63., 66., 69.],
                Shape(vec![3, 3])
            )
            .unwrap()
        );

        assert_eq!(
            sfs.marginalize_axis(Axis(2)),
            Sfs::new(
                vec![3., 12., 21., 30., 39., 48., 57., 66., 75.],
                Shape(vec![3, 3])
            )
            .unwrap()
        );
    }

    #[test]
    fn test_marginalize_3d() {
        let sfs = Sfs::from_range(0..27, Shape(vec![3, 3, 3])).unwrap();

        let expected = Sfs::new(vec![90., 117., 144.], Shape(vec![3])).unwrap();
        assert_eq!(sfs.marginalize(&[Axis(0), Axis(2)]).unwrap(), expected);
        assert_eq!(sfs.marginalize(&[Axis(2), Axis(0)]).unwrap(), expected);
    }

    #[test]
    fn test_marginalize_too_many_axes() {
        let sfs = Sfs::from_range(0..9, Shape(vec![3, 3])).unwrap();

        assert_eq!(
            sfs.marginalize(&[Axis(0), Axis(1)]),
            Err(MarginalizationError::TooManyAxes {
                axes: 2,
                dimensions: 2
            }),
        );
    }

    #[test]
    fn test_marginalize_duplicate_axis() {
        let sfs = Sfs::from_range(0..27, Shape(vec![3, 3, 3])).unwrap();

        assert_eq!(
            sfs.marginalize(&[Axis(1), Axis(1)]),
            Err(MarginalizationError::DuplicateAxis { axis: 1 }),
        );
    }

    #[test]
    fn test_marginalize_axis_out_ouf_bounds() {
        let sfs = Sfs::from_range(0..9, Shape(vec![3, 3])).unwrap();

        assert_eq!(
            sfs.marginalize(&[Axis(2)]),
            Err(MarginalizationError::AxisOutOfBounds {
                axis: 2,
                dimensions: 2
            }),
        );
    }
}
