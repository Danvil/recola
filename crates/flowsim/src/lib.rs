mod chunk;
mod flow_net;
mod flow_net_solver;
mod fluid;
mod ode;
mod pipe;
mod pipe_vessel;
mod reservoire_vessel;

pub mod models;

pub use chunk::*;
pub use flow_net::*;
pub use flow_net_solver::*;
pub use fluid::*;
pub use ode::*;
pub use pipe::*;
pub use pipe_vessel::*;
pub use reservoire_vessel::*;

pub trait Mix: Sized {
    fn mix(a: &Self, b: &Self) -> Self;

    fn scale(a: &Self, s: f64) -> Self;

    fn split(&self, q: f64) -> (Self, Self) {
        (Self::scale(self, q), Self::scale(self, 1. - q))
    }

    fn mix_many<'a, I>(mut iter: I) -> Option<Self>
    where
        I: Iterator<Item = &'a Self>,
        Self: Clone + 'a,
    {
        let mut a = iter.next()?.clone();
        for x in iter {
            a = Self::mix(&a, x);
        }
        Some(a)
    }
}
