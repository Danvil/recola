use crate::{BloodOxygenContent, FlowSimMocca, HemoglobinOxygenSaturationHill, TimeMocca, Tissue};
use flecs_ecs::prelude::*;
use gems::Lerp;
use mocca::{Mocca, MoccaDeps};
use std::sync::Arc;

/// Blood carries oxygen, nutrients and pollutants between body parts.
#[derive(Component)]
pub struct BloodMocca;

#[derive(Component, Clone)]
pub struct BloodConfig {
    pub hill: Arc<HemoglobinOxygenSaturationHill<f64>>,
    pub o2_content: Arc<BloodOxygenContent<f64>>,

    /// L O2 / L blood / mmHg / s
    pub transport_factor: f64,
}

/// Properties of a chunk of blood
#[derive(Clone)]
pub struct BloodProperties {
    pub hill_model: Arc<HemoglobinOxygenSaturationHill<f64>>,
    pub o2_content_model: Arc<BloodOxygenContent<f64>>,

    /// Hematocrit [%], i.e. percentage of blood which is red blood cells (RBC)
    // TODO make private to guarantee set_o2_content is correct
    pub hematocrit: f64,

    /// O₂ content of blood [L/L], i.e. liter O2 per liter of blood
    o2_content: f64,

    /// O2 partial pressure [Pa]
    po2: f64,

    /// O2 saturation [%]
    so2: f64,

    /// Blood glucose concentration [g/L]
    pub glucose_level: f64,
}

impl BloodProperties {
    pub fn new(cfg: &BloodConfig, hematocrit: f64, o2_content: f64, glucose_level: f64) -> Self {
        let mut out = BloodProperties {
            hill_model: cfg.hill.clone(),
            o2_content_model: cfg.o2_content.clone(),
            hematocrit,
            o2_content,
            po2: 0.,
            so2: 0.,
            glucose_level,
        };
        out.set_o2_content(o2_content);
        out
    }

    pub fn po2(&self) -> f64 {
        self.po2
    }

    pub fn so2(&self) -> f64 {
        self.so2
    }

    pub fn o2_content(&self) -> f64 {
        self.o2_content
    }

    pub fn set_o2_content(&mut self, o2_content: f64) {
        self.o2_content = o2_content.max(0.01);

        // Compute O2 saturation (simplified assumption that all O2 is bound to Hb)
        self.so2 = self
            .o2_content_model
            .hb_o2_content_into_so2(self.hematocrit, self.o2_content)
            .min(0.99);

        // Limit SO2 to 99% and adapt the O2 content accordingly
        self.o2_content = self
            .o2_content_model
            .hemoglobin_bound(self.hematocrit, self.so2);

        self.po2 = self.hill_model.saturation_into_pressure(self.so2);
    }
}

impl Lerp<f64> for BloodProperties {
    fn lerp_impl(&self, q: f64, other: &Self) -> Self {
        Self {
            hill_model: self.hill_model.clone(),
            o2_content_model: self.o2_content_model.clone(),
            hematocrit: self.hematocrit.lerp(q, &other.hematocrit),
            o2_content: self.o2_content.lerp(q, &other.o2_content),
            po2: 0., // computed during normalization
            so2: 0., // computed during normalization
            glucose_level: self.glucose_level.lerp(q, &other.glucose_level),
        }
    }

    fn normalize(&mut self) {
        // PO2 and SO2 are not linear (I think ..)
        self.set_o2_content(self.o2_content);
    }
}

#[derive(Component, Default, Clone)]
pub struct BloodStats {
    /// O2 saturation [%]
    pub so2: f64,

    /// O2 partial pressure [Pa]
    pub po2: f64,
}

impl Mocca for BloodMocca {
    fn load(mut dep: MoccaDeps) {
        dep.dep::<TimeMocca>();
        dep.dep::<FlowSimMocca>();
    }

    fn register_components(world: &World) {
        world.component::<BloodConfig>();
        world.component::<BloodStats>();
    }

    fn start(world: &World) -> Self {
        world.set(BloodConfig {
            hill: HemoglobinOxygenSaturationHill::default().into(),
            o2_content: BloodOxygenContent::default().into(),
            transport_factor: 0.002,
        });

        Self
    }

    fn step(&mut self, _world: &World) {
        // // Exchange nutrients between blood and tissue
        // world
        //     .query::<(&Time, &BloodConfig, &mut FlowNetPipeVessel, &mut Tissue)>()
        //     .singleton_at(0)
        //     .singleton_at(1)
        //     .build()
        //     .each(|(time, cfg, vessel, tissue)| {
        //         for (volume, blood) in vessel.chunk_volume_data_mut() {
        //             diffusion(
        //                 cfg.transport_factor * time.sim_dt_f64(),
        //                 volume,
        //                 blood,
        //                 tissue,
        //             )
        //         }
        //     });

        // // Update blood stats
        // world
        //     .query::<(&FlowNetPipeVessel, &mut BloodStats)>()
        //     .build()
        //     .each(|(vessel, stats)| {
        //         if let Some(avg) = vessel.average_data() {
        //             stats.so2 = avg.so2;
        //             stats.po2 = avg.po2;
        //         } else {
        //             stats.so2 = 0.;
        //             stats.po2 = 0.;
        //         }
        //     });
    }
}

fn diffusion(
    transport_factor: f64,
    blood_volume: f64,
    blood: &mut BloodProperties,
    tissue: &mut Tissue,
) {
    // Imaginary model for tissue identical to blood model
    let tissue_o2_saturation = blood
        .o2_content_model
        .hb_o2_content_into_so2(1., tissue.o2_content);
    let tissue_po2 = blood
        .hill_model
        .saturation_into_pressure(tissue_o2_saturation);

    // Compute transported O2 (L/L)
    let dp = blood.po2() - tissue_po2;
    let delta = transport_factor * dp;

    // Move oxygen
    blood.set_o2_content(blood.o2_content() - delta * tissue.volume);
    tissue.o2_content += delta * blood_volume;
}
