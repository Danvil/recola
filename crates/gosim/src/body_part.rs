use crate::{
    stat_component, BloodMocca, BloodOxygenContent, BloodStats, EntityBuilder,
    HemoglobinOxygenSaturationHill, PipeBuilder, TimeMocca,
};
use flecs_ecs::prelude::*;
use flowsim::{models::ElasticTube, FluidComposition};
use gems::{Cylinder, VolumeModel};
use mocca::{Mocca, MoccaDeps};

#[derive(Component)]
pub struct BodyPartMocca;

stat_component!(BodyPartEfficiency);

#[derive(Component)]
pub struct BodyPartConfig {
    pub hill: HemoglobinOxygenSaturationHill<f64>,
    pub o2_content: BloodOxygenContent<f64>,

    /// O2 consumption of tissue L O2 / L tissue / sec
    pub tissue_oxygen_consumption: f64,
}

#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub enum BodyPart {
    Heart,
    Lungs,
    Torso,
}

/// A block of tissue
#[derive(Component, Clone)]
pub struct Tissue {
    /// Volume of tissue chunk [m^3]
    pub volume: f64,

    /// Intracellular O₂ content [L/L]
    ///
    /// This is an imaginary value which currently uses the same model as blood hemoglobin.
    pub o2_content: f64,

    /// Intracellular glucose reserve [g/L]
    pub glucose_level: f64,

    /// Available cellular ATP-equivalent energy density [J/L]
    pub energy_density: f64,
}

impl Tissue {
    pub fn default_with_volume(volume: f64) -> Self {
        Self {
            volume,
            o2_content: 0.200,
            glucose_level: 0.850,
            energy_density: 1.0,
        }
    }
}

#[derive(Component, Default, Clone)]
pub struct TissueStats {
    pub o2_saturation: f64,
    pub o2_pressure: f64,
}

impl Mocca for BodyPartMocca {
    fn load(mut dep: MoccaDeps) {
        dep.dep::<TimeMocca>();
        dep.dep::<BloodMocca>();
    }

    fn register_components(world: &World) {
        world.component::<BodyPartConfig>();
        world.component::<BodyPart>();
        world.component::<Tissue>();
        world.component::<TissueStats>();
    }

    fn start(world: &World) -> Self {
        BodyPartEfficiency::setup(world);

        world.set(BodyPartConfig {
            hill: HemoglobinOxygenSaturationHill::default(),
            o2_content: BloodOxygenContent::default(),
            tissue_oxygen_consumption: 0.01, // ~10 minute until depletion
        });

        Self
    }

    fn step(&mut self, _world: &World) {
        // // Tissue consumes oxygen
        // world
        //     .query::<(&Time, &BodyPartConfig, &mut Tissue)>()
        //     .singleton_at(0)
        //     .singleton_at(1)
        //     .build()
        //     .each(|(time, cfg, tissue)| {
        //         let delta = time.sim_dt_f64() * cfg.tissue_oxygen_consumption;
        //         tissue.o2_content = (tissue.o2_content - delta).max(0.);
        //     });

        // // Update tissue statistics
        // world
        //     .query::<(&BodyPartConfig, &Tissue, &mut TissueStats)>()
        //     .singleton_at(0)
        //     .build()
        //     .each(|(cfg, tissue, stats)| {
        //         stats.o2_saturation = cfg.o2_content.hb_o2_content_into_so2(1.,
        // tissue.o2_content);         stats.o2_pressure =
        // cfg.hill.saturation_into_pressure(stats.o2_saturation);     });
    }
}

const MEAN_CIRCULATORY_FILLING_PRESSURE: f64 = 800.0; // Pa / 6 mmHg

pub struct BloodVesselBuilder {
    pub tube: ElasticTube,
    pub strand_count: f64,
    pub collapse_pressure: f64,
}

impl EntityBuilder for BloodVesselBuilder {
    fn build<'a>(&self, world: &'a World, entity: EntityView<'a>) -> EntityView<'a> {
        // let blood_config = world.cloned::<&BloodConfig>();
        // FIXME use collapse_pressure
        PipeBuilder {
            tube: self.tube.clone(),
            strand_count: self.strand_count,
            fluid: FluidComposition::blood(1.),
            // data: &BloodProperties::new(&blood_config, 0.45, 0.200, 0.850),
            target_pressure: MEAN_CIRCULATORY_FILLING_PRESSURE,
        }
        .build(world, entity)
        .set(BloodStats::default())
    }
}

/// Create a set of blood vessels
pub fn create_blood_vessels<'a>(
    world: &'a World,
    entity: EntityView<'a>,
    volume: f64,
) -> EntityView<'a> {
    let tube = ElasticTube {
        shape: Cylinder {
            radius: 0.003,
            length: 0.300,
        },
        wall_thickness: 0.0005,
        youngs_modulus: 500_000.0,
    };
    let strand_count = volume / tube.nominal_volume();

    BloodVesselBuilder {
        tube,
        strand_count,
        collapse_pressure: -1_000.,
    }
    .build(world, entity)
}

/// Create a chunk of tissue
pub struct TissueBuilder {
    pub volume: f64,
}

impl EntityBuilder for TissueBuilder {
    fn build<'a>(&self, _world: &'a World, entity: EntityView<'a>) -> EntityView<'a> {
        entity
            .set(Tissue::default_with_volume(self.volume))
            .set(TissueStats::default())
    }
}
