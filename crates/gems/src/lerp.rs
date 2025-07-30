use std::ops::AddAssign;
use std::ops::Div;
use std::ops::Add;
use num_traits::Zero;

pub trait Lerp<K>: Sized {
    /// Required implementation for lerp
    fn lerp_impl(&self, q: K, other: &Self) -> Self;

    /// Computes (1 - q) * self + q * other
    fn lerp(&self, q: K, other: &Self) -> Self {
        let mut out = self.lerp_impl(q, other);
        out.normalize();
        out
    }

    /// Weighted average over items.
    /// Weights must be non-negative and sum of weights must be positive.
    fn weighted_average<'a>(items: impl IntoIterator<Item = (K, &'a Self)>) -> Self
    where
        Self: 'a + Sized + Clone,
        K: Copy +AddAssign +Div<Output = K> + Zero + PartialOrd,
    {
        let mut iter = items.into_iter();
        let (mut sw, sv) = iter.next().expect("must have at least one item");
        let mut sv = sv.clone();
        for (w, v) in iter {
            // (sw*sv + w*v) / (sw + w)
            // = sw / (sw + w) * sv + w / (sw + w) * v
            // => q = w / (sw + w)
            if w > K::zero() {
                sw += w;
                sv = sv.lerp_impl(w / sw, &v);
            }
        }
        sv.normalize();
        sv
    }

    /// Weighted average of two items.
    /// Weights must be non-negative and sum of weights must be positive.
    fn weighted_average_2<'a>((v1, x1): (K, &Self), (v2, x2): (K, &Self)) -> Self
    where K: Copy + Add<Output = K> +Div<Output = K>,
    {
        let q = v2 / (v1 + v2);
        x1.lerp(q, x2)
    }

    /// Renormalizes self after averaging to deal with non-linear quantities
    fn normalize(&mut self) {}
}

impl Lerp<f32> for f32 {
    fn lerp_impl(&self, q: f32, other: &Self) -> Self {
        (1. - q) * self + q * other
    }
}

impl Lerp<f64> for f64 {
    fn lerp_impl(&self, q: f64, other: &Self) -> Self {
        (1. - q) * self + q * other
    }
}

impl Lerp<f64> for () {
    fn lerp_impl(&self, _q: f64, _other: &Self) -> Self {
        ()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_lerp_f64() {
        let actual = f64::weighted_average([(0.2, &0.5), (0.5, &0.8), (0.9, &0.1), (0.4, &1.5)]);
        approx::assert_relative_eq!(actual, 0.595);
    }
}
