use crate::{FluidComposition, FluidDensityViscosity, FluidMassVolume, Mix};

/// A chunk of fluid
#[derive(Clone, Debug)]
pub struct FluidChunk {
    fluid: FluidComposition,
    mass_volume_cache: FluidMassVolume,
    density_viscosity_cache: FluidDensityViscosity,
}

impl FluidChunk {
    pub fn from_fluid(fluid: FluidComposition) -> Self {
        let mass_volume_cache = fluid.mass_and_volume();
        let density_viscosity_cache = fluid.density_and_viscosity();
        Self {
            fluid,
            mass_volume_cache,
            density_viscosity_cache,
        }
    }

    pub fn from_fluid_with_volume(fluid: FluidComposition, volume: f64) -> Self {
        let mut chunk = Self::from_fluid(fluid);
        chunk.set_volume(volume);
        chunk
    }

    pub fn fluid(&self) -> &FluidComposition {
        &self.fluid
    }

    pub fn volume(&self) -> f64 {
        self.mass_volume_cache.volume
    }

    pub fn mass(&self) -> f64 {
        self.mass_volume_cache.mass
    }

    pub fn density(&self) -> f64 {
        self.density_viscosity_cache.density
    }

    pub fn viscosity(&self) -> f64 {
        self.density_viscosity_cache.viscosity
    }

    pub fn clone_with_volume(&self, new_volume: f64) -> Self {
        Self::scale(self, new_volume / self.volume())
    }

    pub fn split_by_volume(self, first_volume: f64) -> (Self, Self) {
        let second_volume = (self.volume() - first_volume).max(0.);
        let first_volume = self.volume() - second_volume;
        self.split(first_volume / self.volume())
    }

    pub fn split_off_by_volume(&mut self, split_off_volume: f64) -> Self {
        let remaining_volume = (self.volume() - split_off_volume).max(0.);
        let (remaining, other) = self.split(remaining_volume / self.volume());
        *self = remaining;
        other
    }

    pub fn set_volume(&mut self, volume: f64) {
        let factor = volume / self.volume();
        *self = Self::scale(self, factor);
    }
}

impl Mix for FluidChunk {
    fn mix(a: &FluidChunk, b: &FluidChunk) -> FluidChunk {
        FluidChunk::from_fluid(FluidComposition::mix(&a.fluid, &b.fluid))
    }

    fn scale(a: &FluidChunk, s: f64) -> FluidChunk {
        FluidChunk {
            fluid: FluidComposition::scale(&a.fluid, s),
            mass_volume_cache: FluidMassVolume::scale(&a.mass_volume_cache, s),
            density_viscosity_cache: a.density_viscosity_cache.clone(),
        }
    }
}
