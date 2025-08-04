use crate::{chunk::Mix, Fluid, FluidChunk};

/// A vessel stores a single chunk of fluid. Inflow mixes perfectly.
#[derive(Clone, Debug)]
pub struct ReservoirVessel {
    chunk: Option<FluidChunk>,
}

impl Default for ReservoirVessel {
    fn default() -> Self {
        Self { chunk: None }
    }
}

impl ReservoirVessel {
    /// Return true if the vessel does not contain any liquid
    pub fn is_empty(&self) -> bool {
        self.chunk.is_none()
    }

    /// Volume of liquid stored in the vessel
    pub fn volume(&self) -> f64 {
        self.chunk.as_ref().map_or(0., |c| c.volume)
    }

    /// Fluid contained in the reservoir
    pub fn fluid(&self) -> Option<&Fluid> {
        self.chunk.as_ref().map(|c| &c.fluid)
    }

    /// Mix liquid into the vessel
    pub fn fill(&mut self, incoming: FluidChunk) {
        assert!(incoming.volume >= 0.);

        self.chunk = Some(match self.chunk.as_ref() {
            Some(current) => FluidChunk::mix(current, &incoming),
            None => incoming,
        });
    }

    pub fn drain(&mut self, volume: f64) -> Option<FluidChunk> {
        assert!(volume >= 0.);
        if volume == 0. {
            return None;
        }

        let Some(current) = self.chunk.as_mut() else {
            return None;
        };

        if volume >= current.volume {
            return self.chunk.take();
        }

        let mut out = current.clone();
        out.volume = volume;
        current.volume -= volume;

        Some(out)
    }

    pub fn drain_all(&mut self) -> Option<FluidChunk> {
        self.chunk.take()
    }
}
