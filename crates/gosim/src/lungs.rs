use crate::{
    BloodMocca, BloodProperties, PipeConnectionHelper, TissueBuilder, create_blood_vessels,
    ecs::prelude::*, stat_component, utils::EntityBuilder,
};
use candy_time::CandyTimeMocca;
use flowsim::PortTag;
use gems::volume_from_liters;

/// Breathing transfers oxygen (and other components) from the surrounding air into the blood
/// stream.
#[derive(Component)]
pub struct LungsMocca;

stat_component!(
    /// Rate at which lungs absorbe oxygen from air into the blood
    LungOxygenAbsorption
);

stat_component!(
    /// Rate at which lungs absorbe pollution from air
    LungPollutionAbsorption
);

// /// Base properties of a breathing organ
// #[derive(Component, Debug, Clone)]
// pub struct BreathingOrgan {
//     /// Rate at which blood absorbes oxygen from air
//     pub oxygen_absorption: f64,

//     /// Rate at which pollution is absorbed from air
//     pub pollution_absorption: f64,
// }

// /// Current properties of a breathing organ
// #[derive(Component, Debug, Clone)]
// pub struct CurrentBreathingOrgan {
//     pub oxygen_absorption: f64,
//     pub pollution_absorption: f64,
// }

// /// Device providing oxygen
// #[derive(Component, Debug, Clone)]
// pub struct Rebreather {
//     /// Current charge of the device
//     pub charge: f64,

//     /// Oxygen absorption modifier
//     pub oxygen_absorption_mod: Modifier,

//     /// Pollution absorption modifier
//     pub pollution_absorption_mod: Modifier,
// }

#[derive(Singleton)]
pub struct LungsConfig {
    pub oxygen_diffusion_rate: f64,
    // pub critical_blood_oxygen_value_range: RangeF64,
    // pub critical_blood_oxygen_organ_eff_range: RangeF64,
}

/// Alveoli exchange O2 from air into the blood
#[derive(Component, Clone)]
pub struct Alveoli {
    pub dummy: f64,
}

#[derive(Component)]
pub struct AlveoliTag;

// const CRITICAL_BLOOD_OXYGEN_MOD: &str = "Insufficient Blood Oxygen";
// const REBREATHER_LUNG_OXYGEN_MOD: &str = "Rebreather";

/// Create a pair of standard human lungs
pub fn create_lungs<'a>(
    mut lungs: EntityWorldMut<'a>,
    con: &mut PipeConnectionHelper,
) -> LungsJunctions {
    // TODO create two lungs

    fn part_f<'a>(world: &'a mut World, name: &'static str) -> EntityWorldMut<'a> {
        // TODO make .child_of(lungs);
        let entity = world.spawn(Name::from_str(name));
        world.entity(entity).unwrap()
    }

    // The pulmonary part enriches blood with oxygen

    let pulmonary_in = create_blood_vessels(
        part_f(lungs.world_mut(), "pulmonary_in"),
        volume_from_liters(0.100),
    )
    .id();

    let alveoli = create_blood_vessels(
        part_f(lungs.world_mut(), "alveoli"),
        volume_from_liters(0.300),
    )
    .and_set(Alveoli { dummy: 0. })
    .and_add(AlveoliTag)
    .id();

    let pulmonary_out = create_blood_vessels(
        part_f(lungs.world_mut(), "pulmonary_out"),
        volume_from_liters(0.200),
    )
    .id();

    con.connect_chain(lungs.world_mut(), &[pulmonary_in, alveoli, pulmonary_out]);

    // The bronchial blood supply provides nutrient blood to the lungs

    let bronchial_in = create_blood_vessels(
        part_f(lungs.world_mut(), "bronchial_in"),
        volume_from_liters(0.050),
    )
    .id();

    let bronchial = create_blood_vessels(
        part_f(lungs.world_mut(), "bronchial"),
        volume_from_liters(0.150),
    );
    let bronchial = TissueBuilder { volume: 1. }.build(bronchial).id();

    let bronchial_out = create_blood_vessels(
        part_f(lungs.world_mut(), "bronchial_out"),
        volume_from_liters(0.100),
    )
    .id();

    con.connect_chain(lungs.world_mut(), &[bronchial_in, bronchial, bronchial_out]);

    LungsJunctions {
        pulmonary_in: con.connect_to_new_junction(lungs.world_mut(), (pulmonary_in, PortTag::A)),
        pulmonary_out: con.connect_to_new_junction(lungs.world_mut(), (pulmonary_out, PortTag::B)),
        bronchial_in: con.connect_to_new_junction(lungs.world_mut(), (bronchial_in, PortTag::A)),
        bronchial_out: con.connect_to_new_junction(lungs.world_mut(), (bronchial_out, PortTag::B)),
    }
}

#[derive(Component, Clone)]
pub struct LungsJunctions {
    /// Pulmonary intake (blue)
    pub pulmonary_in: Entity,

    /// Pulmonary output (red)
    pub pulmonary_out: Entity,

    /// Nutrient supply blood inflow
    pub bronchial_in: Entity,

    /// Nutrient supply blood outflow
    pub bronchial_out: Entity,
}

impl Mocca for LungsMocca {
    fn load(mut dep: MoccaDeps) {
        dep.depends_on::<CandyTimeMocca>();
        dep.depends_on::<BloodMocca>();
    }

    fn register_components(world: &mut World) {
        // world.register_component::<BreathingOrgan>();
        // world.register_component::<CurrentBreathingOrgan>();
        world.register_component::<LungsConfig>();
        world.register_component::<AlveoliTag>();
        // world.register_component::<Air>();
        world.register_component::<Alveoli>();
        // world.register_component::<Rebreather>();
        LungOxygenAbsorption::register_components(world);
        LungPollutionAbsorption::register_components(world);
    }

    fn start(world: &mut World) -> Self {
        // Initialize configuration
        world.set_singleton(LungsConfig {
            oxygen_diffusion_rate: 0.000002,
            // blood_oxygen_range: RangeF64::new(0., 150.0),
            // critical_blood_oxygen_value_range: RangeF64::new(0., 40.0),
            // critical_blood_oxygen_organ_eff_range: RangeF64::new(0., 1.0),
        });

        Self
    }

    fn step(&mut self, world: &mut World) {
        LungOxygenAbsorption::step(world);
        LungPollutionAbsorption::step(world);

        // // Alevoli exchange oxygen from air into the blood
        // world
        //     .system_named::<(&Time, &LungsConfig, &Air, &Alveoli, &mut BloodVessel)>(
        //         "LungsAlveoliExchange",
        //     )
        //     .singleton_at(0)
        //     .singleton_at(1)
        //     .term_at(2)
        //     .up()
        //     .each(|(time, cfg, air, _alveoli, vessel)| {
        //         let air_po2 = air.oxygen_percent * 713.0 * 133.322; // Pa
        //         let vol_by_pressure = cfg.oxygen_diffusion_rate * time.sim_dt_f64();
        //         for (_, blood) in vessel.chunk_volume_data_mut() {
        //             alevoli_diffusion(blood, air_po2, vol_by_pressure);
        //             // TODO
        //         }
        //     });

        // // Consume blood oxygen
        // world
        //     .system::<(&BloodOxygen, &mut CurrentBloodOxygen)>()
        //     .each(|(boxy, curr)| {
        //         **curr = (**curr - boxy.rate).max(0.);
        //     });

        // // Low blood oxygen will decrease organ efficiency
        // world
        //     .system::<(
        //         &LungsConfig,
        //         &CurrentBloodOxygen,
        //         &mut OrganEfficiencyMod,
        //     )>()
        //     .singleton_at(0)
        //     .each(|(cfg, oxy, omod)| {
        //         let efficiency = oxy.rescale_clamped(
        //             &cfg.critical_blood_oxygen_value_range,
        //             &cfg.critical_blood_oxygen_organ_eff_range,
        //         );
        //         omod.set_more_mod(CRITICAL_BLOOD_OXYGEN_MOD, efficiency - 1);
        //     });

        // // Update breathing organ
        // world
        //     .system::<(&BreathingOrgan, &mut CurrentBreathingOrgan)>()
        //     .each(|(base, curr)| {
        //         curr.oxygen_absorption = base.oxygen_absorption;
        //         curr.pollution_absorption = base.pollution_absorption;
        //     });

        // // Apply bonus from rebreather
        // // Organ($), Rebreather($item), CurrentOrgan($)
        // // >> Inventory($, $container), ContainedBy($item, $container),
        // world
        //     .system::<(&BreathingOrgan, &mut Rebreather, &mut CurrentBreathingOrgan)>()
        //     .related(This, HasInventory, "$container")
        //     .related("$item", ContainedBy, "$container")
        //     .tagged("$item", Arg(1))
        //     .each(|(base, mask, curr)| {
        //         curr.oxygen_absorption =
        //             base.oxygen_absorption * mask.oxygen_absorption_mod.factor();
        //         curr.pollution_absorption =
        //             base.pollution_absorption * mask.pollution_absorption_mod.factor();
        //     });

        // // Apply bonus from rebreather
        // // Rebreather($item), LungOxygenAbsorptionMod($)
        // // >> Inventory($, $container), ContainedBy($item, $container),
        // world
        //     .system::<(
        //         &Rebreather,
        //         &mut LungOxygenAbsorptionMods,
        //         &mut LungPollutionAbsorptionMods,
        //     )>()
        //     .related(This, HasInventory, "$container")
        //     .related("$item", ContainedBy, "$container")
        //     .tagged("$item", Arg(0))
        //     .each(|(mask, loam, lpam)| {
        //         loam.set_mod(REBREATHER_LUNG_OXYGEN_MOD, mask.oxygen_absorption_mod);
        //         lpam.set_mod(REBREATHER_LUNG_OXYGEN_MOD, mask.pollution_absorption_mod);
        //     });

        // // Accumulate pollution from breathing
        // // Air($location), CurrentBreathingOrgan($), BodyTox($),
        // // + LocatedIn($, $location)
        // world
        //     .system::<(&Air, &LungPollutionAbsorption, &mut BodyTox)>()
        //     .related(This, LocatedIn, "$location")
        //     .tagged("$location", Arg(0))
        //     .each(move |(air, rate, tox)| {
        //         tox.amount += **rate * air.pollution;
        //     });

        // // Restore blood oxygen through breathing
        // // Air($location), CurrentBreathingOrgan($), CurrentBloodOxygen($),
        // // + LocatedIn($, $location)
        // world
        //     .system::<(
        //         &LungsConfig,
        //         &Air,
        //         &LungOxygenAbsorption,
        //         &mut CurrentBloodOxygen,
        //     )>()
        //     .singleton_at(0)
        //     .related(This, LocatedIn, "$location")
        //     .tagged("$location", Arg(0))
        //     .each(|(cfg, air, rate, curr)| {
        //         curr.add_assign_clamp(**rate * air.oxygen_percent, &cfg.blood_oxygen_range);
        //     });

        // // Restore blood oxygen through rebreather
        // // CurrentBreathingOrgan($), Rebreather($item), CurrentBloodOxygen($)
        // // >> Inventory($, $container), ContainedBy($item, $container),
        // world
        //     .system::<(
        //         &LungOxygenAbsorption,
        //         &mut Rebreather,
        //         &mut CurrentBloodOxygen,
        //     )>()
        //     .related(This, HasInventory, "$container")
        //     .related("$item", ContainedBy, "$container")
        //     .tagged("$item", Arg(1))
        //     .each(|(rate, mask, curr)| {
        //         mask.charge -= 1.;
        //         **curr += **rate;
        //     });
    }
}

fn alevoli_diffusion(blood: &mut BloodProperties, air_po2: f64, vol_by_pressure: f64) {
    let dp = air_po2 - blood.po2();
    let do2 = vol_by_pressure * dp;
    blood.set_o2_content(blood.o2_content() + do2);
}
