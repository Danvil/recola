use crate::{JunctionScratch, PipeScratch, PipeState, PortTag};
use excess::prelude::*;
use gems::volume_to_liters;
use simplecs::prelude::*;
use std::error::Error;

pub fn print_junction_overview(query: Query<(This, &JunctionScratch)>) {
    println!(">> Junctions:");
    println!("  {:<6} {:>12} ", "ID", "Pressure [Pa]");
    println!("{}", "-".repeat(6 + 12 * 4 + 6));

    for (id, j) in query.iter() {
        println!("  {:<6} {:?}", id, j.pressure,);
    }
}

pub fn print_pipe_overview(query: Query<(This, Option<&Name>, &PipeState, &PipeScratch)>) {
    // Print header
    println!(">> Pipes:");
    println!(
        "  {:<6} {:>16} {:>9} {:>9} {:>9} {:>9} {:>9} {:>9} {:>9} {:>9} {:>9} {:>9} {:>9} {:>9}",
        "ID",
        "Name",
        "Volume [L]",
        "P_elas",
        "Drag F A",
        "Drag F B",
        "Ext F A",
        "Ext F B",
        "Junc F A",
        "Junc F B",
        "Vel. A [m/s]",
        "Vel. B [m/s]",
        "Area A [cm2]",
        "Area B [cm2]"
    );
    println!("{}", "-".repeat(6 + 16 + 12 * 9 + 15)); // separator

    // Print each pipe
    for (id, name, state, scr) in query.iter() {
        println!(
                "  {:<6} {:>16} {:>9.6} {:>9.3} {:>9.3} {:>9.3} {:>9.3} {:>9.3} {:>9.3} {:>9.3} {:>9.3} {:>9.3} {:>9.3} {:>9.3}",
                id,
                name.unwrap_or_str("N/A"),
                volume_to_liters(state.volume),
                scr.elas_pressure,
                scr.drag_forces[0],
                scr.drag_forces[1],
                scr.ext_accels[0] * scr.mass,
                scr.ext_accels[1] * scr.mass,
                scr.junction_pressure[0].unwrap_or(0.) * scr.area_per_mass,
                scr.junction_pressure[1].unwrap_or(0.) * scr.area_per_mass,
                state.velocity[PortTag::A],
                state.velocity[PortTag::B],
                10000. * scr.port_cross_section_area[0],
                10000. * scr.port_cross_section_area[1],
            );
    }

    println!(
        "Total Volume: {} L",
        volume_to_liters(
            query
                .iter()
                .map(|(_, _, state, _)| state.volume)
                .sum::<f64>()
        )
    )
}

pub fn write_pipes_to_csv(_world: &World, _path: &str) -> Result<(), Box<dyn Error>> {
    todo!();

    // let file = File::create(path)?;
    // let mut writer = BufWriter::new(file);

    // // CSV header
    // writeln!(writer, "ID,Volume,FA,FB,vA,vB")?;

    // // Iterate over pipe states
    // for (id, state) in self.scratch.pipes.borrow().iter() {
    //     let pipe = &self.net.pipes[id];
    //     writeln!(
    //         writer,
    //         "{},{:.6},{:.6},{:.6},{:.6},{:.6}",
    //         id,
    //         pipe.fluid.volume,
    //         state.force[0],
    //         state.force[1],
    //         pipe.velocity[0],
    //         pipe.velocity[1],
    //     )?;
    // }

    // Ok(())
}
