use crate::PipeId;
use std::ops::{Add, Index, IndexMut, Mul, Sub};

/// A pipe has two ports
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum PortTag {
    A,
    B,
}

impl PortTag {
    pub fn index(&self) -> usize {
        match self {
            PortTag::A => 0,
            PortTag::B => 1,
        }
    }

    pub fn opposite(&self) -> PortTag {
        match self {
            PortTag::A => PortTag::B,
            PortTag::B => PortTag::A,
        }
    }

    pub fn tag(&self) -> &'static str {
        match self {
            PortTag::A => "A",
            PortTag::B => "B",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Port {
    PipeOutlet { pipe_id: PipeId, side: PortTag },
}

#[derive(Clone, Copy, Debug, Default)]
pub struct PortMap<T>([T; 2]);

impl<T> PortMap<T> {
    pub fn from_array(array: [T; 2]) -> Self {
        Self(array)
    }
}

impl<T> Index<PortTag> for PortMap<T> {
    type Output = T;

    fn index(&self, side: PortTag) -> &T {
        &self.0[side.index()]
    }
}

impl<T> IndexMut<PortTag> for PortMap<T> {
    fn index_mut(&mut self, side: PortTag) -> &mut T {
        &mut self.0[side.index()]
    }
}

impl<T> Index<usize> for PortMap<T> {
    type Output = T;

    fn index(&self, side: usize) -> &T {
        &self.0[side]
    }
}

impl<T> IndexMut<usize> for PortMap<T> {
    fn index_mut(&mut self, side: usize) -> &mut T {
        &mut self.0[side]
    }
}

impl<T: Copy + Add<Output = T>> Add for PortMap<T> {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        PortMap([self[0] + rhs[0], self[1] + rhs[1]])
    }
}

impl<T: Copy + Sub<Output = T>> Sub for PortMap<T> {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        PortMap([self[0] - rhs[0], self[1] - rhs[1]])
    }
}

impl<T: Copy + Mul<f64, Output = T>> Mul<f64> for PortMap<T> {
    type Output = Self;
    fn mul(self, rhs: f64) -> Self {
        PortMap([self[0] * rhs, self[1] * rhs])
    }
}

impl<T: Copy + Add<Output = T>> Add for &PortMap<T> {
    type Output = PortMap<T>;
    fn add(self, rhs: Self) -> Self::Output {
        PortMap([self[0] + rhs[0], self[1] + rhs[1]])
    }
}

impl<T: Copy + Sub<Output = T>> Sub for &PortMap<T> {
    type Output = PortMap<T>;
    fn sub(self, rhs: Self) -> Self::Output {
        PortMap([self[0] - rhs[0], self[1] - rhs[1]])
    }
}

impl<T: Copy + Mul<f64, Output = T>> Mul<f64> for &PortMap<T> {
    type Output = PortMap<T>;
    fn mul(self, rhs: f64) -> Self::Output {
        PortMap([self[0] * rhs, self[1] * rhs])
    }
}
