use crate::{FluidChunk, Mix, PortTag};
use simplecs::prelude::*;
use std::collections::VecDeque;

/// A fluid vessel which with two ends which stores chunks as a directed list. Fluid can flow in
/// and out on both sides and order is preserved.
#[derive(Component, Clone, Debug)]
pub struct PipeVessel {
    /// Fluid chunks currently contained by the vessels
    chunks: VecDeque<FluidChunk>,

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

    pub fn volume(&self) -> f64 {
        self.combined_chunk().map_or(0., |c| c.volume())
    }

    /// Chunk representing all fluid
    pub fn combined_chunk(&self) -> Option<FluidChunk> {
        FluidChunk::mix_many(self.chunks.iter())
    }

    pub fn chunks(&self) -> impl ExactSizeIterator<Item = &FluidChunk> {
        self.chunks.iter()
    }

    // pub fn chunk_volume_data_mut(&mut self) -> impl Iterator<Item = (f64, &mut Fluid)> {
    //     self.chunks.iter_mut().map(|c| (c.volume(), &mut c.fluid()))
    // }

    /// Push liquid into the pipe at given port
    pub fn fill(&mut self, port: PortTag, chunk: FluidChunk) {
        assert!(chunk.volume() >= 0.);
        if chunk.volume() == 0. {
            return;
        }

        let mut port = PortOp(port, &mut self.chunks);

        let chunk = if let Some(last) = port.get() {
            if last.volume() < self.min_chunk_volume {
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
        }

        impl<'a> Iterator for DrainIter<'a> {
            type Item = FluidChunk;

            fn next(&mut self) -> Option<Self::Item> {
                if self.target <= 0. {
                    return None;
                }

                let next = self.port.pop()?;

                if next.volume() > self.target {
                    // Split chunk
                    let (taken, remainder) = next.split_by_volume(self.target);
                    self.port.push(remainder);
                    self.target = 0.;
                    Some(taken)
                } else {
                    // Take whole chunk
                    self.target -= next.volume();
                    Some(next)
                }
            }
        }

        DrainIter {
            port: PortOp(port, &mut self.chunks),
            target: volume,
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
