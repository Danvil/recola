use super::TransformDisplay;
use glam::{Vec3, Quat};

/// Integration helpers for converting candy Transform types to TransformDisplay
/// 
/// Since we can't directly access the candy crate's Transform3 and GlobalTransform3 types,
/// this module provides conversion functions that can be used when the candy crate is available.

/// Trait for converting transform types to TransformDisplay
pub trait ToTransformDisplay {
    fn to_transform_display(&self, is_global: bool) -> TransformDisplay;
}

/// Generic implementation for any type that has translation, rotation, and scale fields
/// This can be used with Transform3 from the candy crate
pub fn transform_to_display<T>(
    transform: &T,
    is_global: bool,
) -> TransformDisplay
where
    T: HasTransformFields,
{
    TransformDisplay {
        translation: transform.get_translation(),
        rotation: transform.get_rotation(),
        scale: transform.get_scale(),
        is_global,
    }
}

/// Trait for types that have transform fields (translation, rotation, scale)
pub trait HasTransformFields {
    fn get_translation(&self) -> Vec3;
    fn get_rotation(&self) -> Quat;
    fn get_scale(&self) -> Vec3;
}

/// Example implementation for a generic transform struct
/// This shows how to implement the trait for Transform3-like types
#[derive(Debug, Clone)]
pub struct GenericTransform {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl HasTransformFields for GenericTransform {
    fn get_translation(&self) -> Vec3 {
        self.translation
    }

    fn get_rotation(&self) -> Quat {
        self.rotation
    }

    fn get_scale(&self) -> Vec3 {
        self.scale
    }
}

impl ToTransformDisplay for GenericTransform {
    fn to_transform_display(&self, is_global: bool) -> TransformDisplay {
        transform_to_display(self, is_global)
    }
}

/// Utility functions for common transform operations in the GUI
pub mod gui_utils {
    use super::*;

    /// Create a TransformDisplay from individual components
    pub fn create_transform_display(
        translation: Vec3,
        rotation: Quat,
        scale: Vec3,
        is_global: bool,
    ) -> TransformDisplay {
        if is_global {
            TransformDisplay::new_global(translation, rotation, scale)
        } else {
            TransformDisplay::new_local(translation, rotation, scale)
        }
    }

    /// Format multiple transforms for side-by-side comparison
    pub fn format_transform_comparison(
        local: &TransformDisplay,
        global: &TransformDisplay,
    ) -> Vec<String> {
        let mut result = Vec::new();
        
        result.push("┌─ Transform Comparison ─────────────────────────────────────┐".to_string());
        result.push("│ Local Transform              │ Global Transform             │".to_string());
        result.push("├──────────────────────────────┼──────────────────────────────┤".to_string());
        
        result.push(format!(
            "│ {:<28} │ {:<28} │",
            local.format_translation(),
            global.format_translation()
        ));
        
        result.push(format!(
            "│ {:<28} │ {:<28} │",
            local.format_rotation(),
            global.format_rotation()
        ));
        
        result.push(format!(
            "│ {:<28} │ {:<28} │",
            local.format_scale(),
            global.format_scale()
        ));
        
        result.push("└──────────────────────────────┴──────────────────────────────┘".to_string());
        
        result
    }

    #[cfg(feature = "egui")]
    pub fn render_transform_comparison_egui(
        ui: &mut egui::Ui,
        local: &TransformDisplay,
        global: &TransformDisplay,
    ) {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.heading("Local vs Global Transform");
                ui.separator();
                
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        local.render_egui(ui);
                    });
                    
                    ui.separator();
                    
                    ui.vertical(|ui| {
                        global.render_egui(ui);
                    });
                });
            });
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generic_transform_conversion() {
        let transform = GenericTransform {
            translation: Vec3::new(1.0, 2.0, 3.0),
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        };

        let display = transform.to_transform_display(false);
        assert_eq!(display.translation, Vec3::new(1.0, 2.0, 3.0));
        assert!(!display.is_global);
    }

    #[test]
    fn test_transform_comparison_formatting() {
        let local = TransformDisplay::new_local(
            Vec3::new(1.0, 2.0, 3.0),
            Quat::IDENTITY,
            Vec3::ONE,
        );
        
        let global = TransformDisplay::new_global(
            Vec3::new(4.0, 5.0, 6.0),
            Quat::IDENTITY,
            Vec3::ONE,
        );

        let comparison = gui_utils::format_transform_comparison(&local, &global);
        assert!(comparison.len() > 5);
        assert!(comparison[0].contains("Transform Comparison"));
    }
}
