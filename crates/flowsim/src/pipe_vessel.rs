use crate::{chunk::Mix, Fluid, FluidChunk};
use std::collections::VecDeque;

/// A fluid vessel which with two ends which stores chunks as a directed list. Fluid can flow in
/// and out on both sides and order is preserved.
#[derive(Clone, Debug)]
pub struct PipeVessel {
    /// Fluid chunks currently contained by the vessels
    chunks: VecDeque<FluidChunk>,

    /// Total volume of all chunks
    volume: f64,

    /// Chunks smaller than this will be merged.
    min_chunk_volume: f64,
}

impl Default for PipeVessel {
    fn default() -> Self {
        Self::new()
    }
}

impl PipeVessel {
    pub fn new() -> Self {
        Self {
            chunks: VecDeque::new(),
            volume: 0.,
            min_chunk_volume: 0.,
        }
    }

    pub fn set_min_chunk_volume(&mut self, min_chunk_volume: f64) {
        self.min_chunk_volume = min_chunk_volume;
    }

    pub fn with_min_chunk_volume(mut self, min_chunk_volume: f64) -> Self {
        self.set_min_chunk_volume(min_chunk_volume);
        self
    }

    /// Volume of liquid stored in the pipe
    pub fn volume(&self) -> f64 {
        self.volume
    }

    /// Volume-weighted average chunk data
    pub fn average_data(&self) -> Option<Fluid> {
        if self.volume == 0. {
            None
        } else {
            Fluid::mix_many(self.chunks.iter().map(|c| &c.fluid))
        }
    }

    pub fn chunks(&self) -> impl ExactSizeIterator<Item = &FluidChunk> {
        self.chunks.iter()
    }

    pub fn chunk_volume_data_mut(&mut self) -> impl Iterator<Item = (f64, &mut Fluid)> {
        self.chunks.iter_mut().map(|c| (c.volume, &mut c.fluid))
    }

    /// Push liquid into the pipe at given port
    pub fn fill(&mut self, port: PortTag, chunk: FluidChunk) {
        assert!(chunk.volume >= 0.);
        if chunk.volume == 0. {
            return;
        }

        self.volume += chunk.volume;

        let mut port = PortOp(port, &mut self.chunks);

        let chunk = if let Some(last) = port.get() {
            if last.volume < self.min_chunk_volume {
                // last chunk too small - mix in the inflow
                let mix = FluidChunk::mix(last, &chunk);
                port.pop();
                mix
            } else {
                // start new chunk
                chunk
            }
        } else {
            // first chunk
            chunk
        };

        port.push(chunk);
    }

    pub fn filled(mut self, port: PortTag, chunk: FluidChunk) -> Self {
        self.fill(port, chunk);
        self
    }

    /// Drain fluid from the pipe at given port.
    pub fn drain(&mut self, port: PortTag, volume: f64) -> impl Iterator<Item = FluidChunk> + '_ {
        assert!(volume >= 0.);

        struct DrainIter<'a> {
            port: PortOp<'a, FluidChunk>,
            target: f64,
            volume_ref: &'a mut f64,
        }

        impl<'a> Iterator for DrainIter<'a> {
            type Item = FluidChunk;

            fn next(&mut self) -> Option<Self::Item> {
                if self.target <= 0. {
                    return None;
                }

                let next = self.port.pop()?;

                if next.volume > self.target {
                    // Split chunk
                    let mut remainder = next.clone();
                    remainder.volume -= self.target;
                    self.port.push(remainder);

                    let mut taken = next;
                    taken.volume = self.target;
                    *self.volume_ref -= self.target;
                    self.target = 0.;
                    Some(taken)
                } else {
                    // Take whole chunk
                    *self.volume_ref -= next.volume;
                    self.target -= next.volume;
                    Some(next)
                }
            }
        }

        DrainIter {
            port: PortOp(port, &mut self.chunks),
            target: volume,
            volume_ref: &mut self.volume,
        }
    }
}

/// Helper type to work on the ports of a pipe
struct PortOp<'a, T>(PortTag, &'a mut VecDeque<T>);

impl<'a, T> PortOp<'a, T> {
    /// Current chunk at port
    fn get(&self) -> Option<&T> {
        match self.0 {
            PortTag::A => self.1.front(),
            PortTag::B => self.1.back(),
        }
    }

    /// Pop chunk from port
    fn pop(&mut self) -> Option<T> {
        match self.0 {
            PortTag::A => self.1.pop_front(),
            PortTag::B => self.1.pop_back(),
        }
    }

    /// Push chunk into port
    fn push(&mut self, chunk: T) {
        match self.0 {
            PortTag::A => self.1.push_front(chunk),
            PortTag::B => self.1.push_back(chunk),
        }
    }
}

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
