use super::{TransformDisplay, integration::*};
use glam::{Vec3, Quat};

/// Examples demonstrating the improved Transform display functionality
/// 
/// This module shows how to use the TransformDisplay system to create
/// better-looking Transform and GlobalTransform displays in forge.

/// Example of creating and displaying transforms
pub fn example_basic_usage() {
    // Create a local transform
    let local_transform = TransformDisplay::new_local(
        Vec3::new(10.5, 20.3, -5.7),
        Quat::from_rotation_y(45.0_f32.to_radians()),
        Vec3::new(1.0, 2.0, 1.0),
    );

    // Create a global transform
    let global_transform = TransformDisplay::new_global(
        Vec3::new(15.2, 25.8, -3.1),
        Quat::from_rotation_y(45.0_f32.to_radians()),
        Vec3::new(1.0, 2.0, 1.0),
    );

    // Print formatted output
    println!("=== Basic Transform Display ===");
    println!("{}", local_transform);
    println!();
    println!("{}", global_transform);
    println!();

    // Print aligned format
    println!("=== Aligned Format ===");
    for line in local_transform.format_aligned() {
        println!("{}", line);
    }
    println!();

    // Print comparison
    println!("=== Side-by-Side Comparison ===");
    let comparison = integration::gui_utils::format_transform_comparison(
        &local_transform,
        &global_transform,
    );
    for line in comparison {
        println!("{}", line);
    }
}

/// Example showing the improvements over the old format
pub fn example_before_after_comparison() {
    let transform = TransformDisplay::new_local(
        Vec3::new(1.234567, 2.345678, 3.456789),
        Quat::from_rotation_xyz(
            30.0_f32.to_radians(),
            45.0_f32.to_radians(),
            60.0_f32.to_radians(),
        ),
        Vec3::new(1.5, 2.0, 0.8),
    );

    println!("=== BEFORE (Old Format - Multiple Labels) ===");
    println!("Transform:");
    println!("  Translation:");
    println!("    X: 1.234567");
    println!("    Y: 2.345678");
    println!("    Z: 3.456789");
    println!("  Rotation:");
    println!("    X: 30.0°");
    println!("    Y: 45.0°");
    println!("    Z: 60.0°");
    println!("  Scale:");
    println!("    X: 1.5");
    println!("    Y: 2.0");
    println!("    Z: 0.8");
    println!();

    println!("=== AFTER (New Format - Clean & Aligned) ===");
    println!("{}", transform);
    println!();

    println!("=== AFTER (Compact Format) ===");
    println!("Local Transform | T: 1.2 2.3 3.5 | R: 30° 45° 60° | S: 1.5 2.0 0.8");
}

/// Example of how to integrate with existing Transform3/GlobalTransform3 types
pub fn example_integration_with_candy_types() {
    // This shows how you would integrate with the actual candy Transform3 types
    // when they become available
    
    println!("=== Integration Example ===");
    println!("// In your forge GUI code:");
    println!("fn display_entity_transforms(entity: Entity, world: &World) {{");
    println!("    if let Some(local_tf) = world.get::<Transform3>(entity) {{");
    println!("        let display = TransformDisplay::new_local(");
    println!("            local_tf.translation,");
    println!("            local_tf.rotation,");
    println!("            local_tf.scale,");
    println!("        );");
    println!("        display.render_egui(ui);");
    println!("    }}");
    println!();
    println!("    if let Some(global_tf) = world.get::<GlobalTransform3>(entity) {{");
    println!("        let display = TransformDisplay::new_global(");
    println!("            global_tf.translation,");
    println!("            global_tf.rotation,");
    println!("            global_tf.scale,");
    println!("        );");
    println!("        display.render_egui(ui);");
    println!("    }}");
    println!("}}");
}

#[cfg(feature = "egui")]
pub mod egui_examples {
    use super::*;
    use egui::{Context, CentralPanel};

    /// Example egui application showing the improved Transform display
    pub fn run_example_app(ctx: &Context) {
        CentralPanel::default().show(ctx, |ui| {
            ui.heading("Improved Transform Display Examples");
            ui.separator();

            // Example transforms
            let local_transform = TransformDisplay::new_local(
                Vec3::new(10.5, 20.3, -5.7),
                Quat::from_rotation_y(45.0_f32.to_radians()),
                Vec3::new(1.0, 2.0, 1.0),
            );

            let global_transform = TransformDisplay::new_global(
                Vec3::new(15.2, 25.8, -3.1),
                Quat::from_rotation_y(45.0_f32.to_radians()),
                Vec3::new(1.0, 2.0, 1.0),
            );

            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.heading("Standard Display");
                    local_transform.render_egui(ui);
                    global_transform.render_egui(ui);
                });

                ui.separator();

                ui.vertical(|ui| {
                    ui.heading("Compact Display");
                    local_transform.render_egui_compact(ui);
                    global_transform.render_egui_compact(ui);
                });
            });

            ui.separator();

            ui.heading("Side-by-Side Comparison");
            integration::gui_utils::render_transform_comparison_egui(
                ui,
                &local_transform,
                &global_transform,
            );
        });
    }
}

/// Performance comparison showing the benefits of the new format
pub fn example_performance_benefits() {
    println!("=== Performance Benefits ===");
    println!("Old Format Issues:");
    println!("- Multiple separate UI elements for each component");
    println!("- Excessive vertical space usage");
    println!("- Poor visual hierarchy");
    println!("- Difficult to scan quickly");
    println!();
    println!("New Format Benefits:");
    println!("- Single line per transform component");
    println!("- Consistent alignment using egui Grid");
    println!("- Clear visual distinction between local/global");
    println!("- Color coding for quick identification");
    println!("- Compact mode for space-constrained views");
    println!("- Better readability with proper precision");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_examples_run_without_panic() {
        // These should not panic
        example_basic_usage();
        example_before_after_comparison();
        example_integration_with_candy_types();
        example_performance_benefits();
    }
}
