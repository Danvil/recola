use crate::{
    create_blood_vessel, create_tissue, stat_component, Air, BloodModule, BloodProperties,
    BloodVessel, FlecsQueryRelationHelpers, FluidFlowLink, Time, TimeModule,
};
use flecs_ecs::prelude::*;

/// Breathing transfers oxygen (and other components) from the surrounding air into the blood
/// stream.
#[derive(Component)]
pub struct LungsModule;

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

#[derive(Component)]
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
pub fn create_lungs<'a>(world: &'a World, lungs: EntityView<'a>) -> LungsSlots<'a> {
    let part_f = |name| world.entity_named(name).child_of(lungs);

    // The pulmonary part enriches blood with oxygen

    let alveoli = create_blood_vessel(&world, part_f("alveoli"), 0.300);
    alveoli.set(Alveoli { dummy: 0. }).add(AlveoliTag);

    // The bronchial blood supply provides nutrient blood to the lungs

    let bronchial_in = create_blood_vessel(&world, part_f("bronchial_in"), 0.050);

    let bronchial = create_blood_vessel(&world, part_f("bronchial"), 0.150);
    create_tissue(bronchial);

    let bronchial_out = create_blood_vessel(&world, part_f("bronchial_out"), 0.100);

    bronchial_in.add((FluidFlowLink, bronchial));
    bronchial.add((FluidFlowLink, bronchial_out));

    LungsSlots {
        lungs,
        alveoli,
        bronchial_in,
        bronchial_out,
    }
}

#[derive(Component, Clone)]
pub struct LungsSlots<'a> {
    pub lungs: EntityView<'a>,

    /// Alevoli vessels (must be connected to pulmory vessels from heart)
    pub alveoli: EntityView<'a>,

    /// Nutrient supply blood inflow
    pub bronchial_in: EntityView<'a>,

    /// Nutrient supply blood outflow
    pub bronchial_out: EntityView<'a>,
}

impl Module for LungsModule {
    fn module(world: &World) {
        world.module::<LungsModule>("breathing");

        world.import::<TimeModule>();
        world.import::<BloodModule>();

        // world.component::<BreathingOrgan>();
        // world.component::<CurrentBreathingOrgan>();
        world.component::<AlveoliTag>();
        world.component::<Air>();
        world.component::<Alveoli>();
        // world.component::<Rebreather>();

        // Initialize configuration
        world.set(LungsConfig {
            oxygen_diffusion_rate: 0.000002,
            // blood_oxygen_range: RangeF64::new(0., 150.0),
            // critical_blood_oxygen_value_range: RangeF64::new(0., 40.0),
            // critical_blood_oxygen_organ_eff_range: RangeF64::new(0., 1.0),
        });

        // Alevoli exchange oxygen from air into the blood
        world
            .system_named::<(&Time, &LungsConfig, &Air, &Alveoli, &mut BloodVessel)>(
                "LungsAlveoliExchange",
            )
            .singleton_at(0)
            .singleton_at(1)
            .term_at(2)
            .up()
            .each(|(time, cfg, air, _alveoli, vessel)| {
                let air_po2 = air.oxygen_percent * 713.0 * 133.322; // Pa
                let vol_by_pressure = cfg.oxygen_diffusion_rate * time.sim_dt_f64();
                for (_, blood) in vessel.chunk_volume_data_mut() {
                    alevoli_diffusion(blood, air_po2, vol_by_pressure);
                    // TODO
                }
            });

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

        LungOxygenAbsorption::setup(world);

        LungPollutionAbsorption::setup(world);

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
