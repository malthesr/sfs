macro_rules! assert_approx_eq {
    ($lhs:expr, $rhs:expr, epsilon = $epsilon:expr) => {
        match (&($lhs), &($rhs)) {
            (lhs, rhs) => assert!(
                $crate::approx::Approx::default()
                    .with($epsilon)
                    .eq(&lhs, &rhs),
                r#"assertion failed: `({} ≈ {})`
  left: `{:?}`,
 right: `{:?}`"#,
                stringify!($lhs),
                stringify!($rhs),
                lhs,
                rhs,
            ),
        }
    };
}

#[derive(Clone, Debug)]
pub struct Approx<T>
where
    T: ApproxEq + ?Sized,
{
    epsilon: T::Epsilon,
}

impl<T> Approx<T>
where
    T: ApproxEq + ?Sized,
{
    pub fn with(mut self, epsilon: T::Epsilon) -> Self {
        self.epsilon = epsilon;
        Self { epsilon }
    }

    pub fn eq(self, lhs: &T, rhs: &T) -> bool {
        T::approx_eq(lhs, rhs, self.epsilon)
    }
}

impl<T> Default for Approx<T>
where
    T: ApproxEq,
{
    fn default() -> Self {
        Self {
            epsilon: T::DEFAULT_EPSILON,
        }
    }
}

pub trait ApproxEq {
    const DEFAULT_EPSILON: Self::Epsilon;

    type Epsilon: Copy + Sized;

    fn approx_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool;
}

impl ApproxEq for f64 {
    const DEFAULT_EPSILON: Self::Epsilon = f64::EPSILON;

    type Epsilon = f64;

    fn approx_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        (self - other).abs() < epsilon
    }
}

impl<T> ApproxEq for [T]
where
    T: ApproxEq,
{
    const DEFAULT_EPSILON: Self::Epsilon = T::DEFAULT_EPSILON;

    type Epsilon = T::Epsilon;

    fn approx_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        self.len() == other.len()
            && self
                .iter()
                .zip(other.iter())
                .all(|(x, y)| ApproxEq::approx_eq(x, y, epsilon))
    }
}

impl<T> ApproxEq for Vec<T>
where
    T: ApproxEq,
{
    const DEFAULT_EPSILON: Self::Epsilon = T::DEFAULT_EPSILON;

    type Epsilon = T::Epsilon;

    fn approx_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        <[T]>::approx_eq(self, other, epsilon)
    }
}

impl<'a, T> ApproxEq for &'a T
where
    T: ApproxEq + ?Sized,
{
    const DEFAULT_EPSILON: Self::Epsilon = T::DEFAULT_EPSILON;

    type Epsilon = T::Epsilon;

    fn approx_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        T::approx_eq(*self, *other, epsilon)
    }
}

impl<'a, T> ApproxEq for &'a mut T
where
    T: ApproxEq + ?Sized,
{
    const DEFAULT_EPSILON: Self::Epsilon = T::DEFAULT_EPSILON;

    type Epsilon = T::Epsilon;

    fn approx_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        T::approx_eq(*self, *other, epsilon)
    }
}
