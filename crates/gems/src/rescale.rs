#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Range<T> {
    pub min: T,
    pub max: T,
}

impl<T: std::cmp::PartialOrd> Range<T> {
    pub fn new(min: T, max: T) -> Self {
        assert!(min <= max, "Range::new: min must be <= max");
        Self { min, max }
    }

    pub fn contains(&self, value: T) -> bool {
        value >= self.min && value <= self.max
    }
}

pub trait RescaleExt: Sized + Clone + Copy {
    fn clamp(self, range: &Range<Self>) -> Self;

    fn add_assign_clamp(self, delta: Self, range: &Range<Self>) -> Self
    where
        Self: core::ops::Add<Output = Self>,
    {
        (self + delta).clamp(range)
    }

    /// Rescale from x_range to y_range
    fn rescale(self, x_range: &Range<Self>, y_range: &Range<Self>) -> Self
    where
        Self: core::ops::Add<Output = Self>
            + core::ops::Sub<Output = Self>
            + core::ops::Mul<Output = Self>
            + core::ops::Div<Output = Self>,
    {
        y_range.min + (y_range.max - y_range.min) * self.rescale_01(x_range)
    }

    /// Rescale from x_range to y_range with enforeced bounds
    fn rescale_clamped(self, x_range: &Range<Self>, y_range: &Range<Self>) -> Self
    where
        Self: core::ops::Add<Output = Self>
            + core::ops::Sub<Output = Self>
            + core::ops::Mul<Output = Self>
            + core::ops::Div<Output = Self>,
    {
        self.clamp(x_range).rescale(x_range, y_range)
    }

    /// Rescale from x_range to [0, 1]
    fn rescale_01(self, range: &Range<Self>) -> Self
    where
        Self: core::ops::Sub<Output = Self> + core::ops::Div<Output = Self>,
    {
        (self - range.min) / (range.max - range.min)
    }

    /// Rescale from x_range to [0, 1] with enforced bounds
    fn rescale_01_clamped(self, range: &Range<Self>) -> Self
    where
        Self: core::ops::Sub<Output = Self> + core::ops::Div<Output = Self>,
    {
        self.clamp(range).rescale_01(range)
    }
}

impl RescaleExt for f64 {
    fn clamp(self, range: &Range<Self>) -> Self {
        if self < range.min {
            range.min
        } else if self > range.max {
            range.max
        } else {
            self
        }
    }
}

pub type RangeF64 = Range<f64>;
