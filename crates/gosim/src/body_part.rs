use crate::{
    stat_component, utils::FlecsQueryRelationHelpers, BloodConfig, BloodModule, BloodOxygenContent,
    BloodProperties, BloodStats, EntityBuilder, HemoglobinOxygenSaturationHill, PipeBuilder,
    PipeGeometry, Time, TimeModule,
};
use flecs_ecs::prelude::*;

#[derive(Component)]
pub struct BodyPartModule;

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
    /// Volume of tissue chunk
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

impl Module for BodyPartModule {
    fn module(world: &World) {
        world.module::<BodyPartModule>("BodyPartModule");

        world.import::<TimeModule>();
        world.import::<BloodModule>();

        world.component::<BodyPartConfig>();
        world.component::<BodyPart>();
        world.component::<Tissue>();
        world.component::<TissueStats>();

        BodyPartEfficiency::setup(world);

        world.set(BodyPartConfig {
            hill: HemoglobinOxygenSaturationHill::default(),
            o2_content: BloodOxygenContent::default(),
            tissue_oxygen_consumption: 0.01, // ~10 minute until depletion
        });

        // Tissue consumes oxygen
        world
            .system::<(&Time, &BodyPartConfig, &mut Tissue)>()
            .singleton_at(0)
            .singleton_at(1)
            .each(|(time, cfg, tissue)| {
                let delta = time.sim_dt_f64() * cfg.tissue_oxygen_consumption;
                tissue.o2_content = (tissue.o2_content - delta).max(0.);
            });

        // Update tissue statistics
        world
            .system::<(&BodyPartConfig, &Tissue, &mut TissueStats)>()
            .singleton_at(0)
            .each(|(cfg, tissue, stats)| {
                stats.o2_saturation = cfg.o2_content.hb_o2_content_into_so2(1., tissue.o2_content);
                stats.o2_pressure = cfg.hill.saturation_into_pressure(stats.o2_saturation);
            });
    }
}

pub struct BloodVesselBuilder<'a> {
    pub geometry: &'a PipeGeometry,
}

impl EntityBuilder for BloodVesselBuilder<'_> {
    fn build<'a>(&self, world: &'a World, entity: EntityView<'a>) -> EntityView<'a> {
        let blood_config = world.cloned::<&BloodConfig>();
        PipeBuilder {
            geometry: self.geometry,
            data: &BloodProperties::new(&blood_config, 0.45, 0.200, 0.850),
            target_pressure: 100. * 133.,
        }
        .build(world, entity)
    }
}

pub fn create_blood_vessel_aux<'a>(
    world: &'a World,
    entity: EntityView<'a>,
    geometry: PipeGeometry,
) -> EntityView<'a> {
    let blood_config = world.cloned::<&BloodConfig>();

    PipeBuilder {
        geometry: &geometry,
        data: &BloodProperties::new(&blood_config, 0.45, 0.200, 0.850),
        target_pressure: 12000.,
    }
    .build(world, entity)
    .set(BloodStats::default())
}

/// Create a set of blood <'a>vessels
pub fn create_blood_vessel<'a>(
    world: &'a World,
    entity: EntityView<'a>,
    volume: f64,
) -> EntityView<'a> {
    let mut geometry = PipeGeometry {
        radius: 0.002,
        length: 0.40,
        wall_thickness: 0.0005,
        youngs_modulus: 500000.,
        count: 1.,
        pressure_min: -5000.,
    };
    geometry.count = geometry.volume_to_count(volume);
    create_blood_vessel_aux(world, entity, geometry)
}

/// Create a chunk of tissue
pub fn create_tissue(entity: EntityView) -> EntityView {
    entity
        .set(Tissue::default_with_volume(1.00))
        .set(TissueStats::default())
}
