use glam::{Vec3, Quat};
use std::fmt;

pub mod integration;
pub mod examples;

/// Improved display formatting for Transform components in forge GUI
pub struct TransformDisplay {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
    pub is_global: bool,
}

impl TransformDisplay {
    pub fn new_local(translation: Vec3, rotation: Quat, scale: Vec3) -> Self {
        Self {
            translation,
            rotation,
            scale,
            is_global: false,
        }
    }

    pub fn new_global(translation: Vec3, rotation: Quat, scale: Vec3) -> Self {
        Self {
            translation,
            rotation,
            scale,
            is_global: true,
        }
    }

    /// Format translation as a single line: "Translation: X.XX Y.YY Z.ZZ"
    pub fn format_translation(&self) -> String {
        format!(
            "Translation: {:.2} {:.2} {:.2}",
            self.translation.x, self.translation.y, self.translation.z
        )
    }

    /// Format rotation as Euler angles in degrees: "Rotation: X.X° Y.Y° Z.Z°"
    pub fn format_rotation(&self) -> String {
        let (x, y, z) = self.rotation.to_euler(glam::EulerRot::XYZ);
        format!(
            "Rotation: {:.1}° {:.1}° {:.1}°",
            x.to_degrees(),
            y.to_degrees(),
            z.to_degrees()
        )
    }

    /// Format scale as a single line: "Scale: X.XX Y.YY Z.ZZ"
    pub fn format_scale(&self) -> String {
        format!(
            "Scale: {:.2} {:.2} {:.2}",
            self.scale.x, self.scale.y, self.scale.z
        )
    }

    /// Get the transform type label with proper indication
    pub fn get_type_label(&self) -> &'static str {
        if self.is_global {
            "Global Transform"
        } else {
            "Local Transform"
        }
    }

    /// Format all transform components with proper alignment
    pub fn format_aligned(&self) -> Vec<String> {
        vec![
            format!("┌─ {} ─", self.get_type_label()),
            format!("│ {}", self.format_translation()),
            format!("│ {}", self.format_rotation()),
            format!("│ {}", self.format_scale()),
            "└─────────────────────".to_string(),
        ]
    }
}

impl fmt::Display for TransformDisplay {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{}", self.get_type_label())?;
        writeln!(f, "  {}", self.format_translation())?;
        writeln!(f, "  {}", self.format_rotation())?;
        write!(f, "  {}", self.format_scale())
    }
}

#[cfg(feature = "egui")]
pub mod egui_integration {
    use super::TransformDisplay;
    use egui::{Ui, Color32, RichText};

    impl TransformDisplay {
        /// Render the transform display using egui with improved layout
        pub fn render_egui(&self, ui: &mut Ui) {
            // Header with clear indication of transform type
            let header_color = if self.is_global {
                Color32::from_rgb(100, 150, 255) // Blue for global
            } else {
                Color32::from_rgb(150, 255, 100) // Green for local
            };

            ui.colored_label(
                header_color,
                RichText::new(self.get_type_label()).strong()
            );

            ui.separator();

            // Use a grid for better alignment
            egui::Grid::new(format!("transform_grid_{}", self.is_global))
                .num_columns(2)
                .spacing([10.0, 4.0])
                .show(ui, |ui| {
                    // Translation row
                    ui.label("Translation:");
                    ui.label(format!("{:.2} {:.2} {:.2}", 
                        self.translation.x, self.translation.y, self.translation.z));
                    ui.end_row();

                    // Rotation row (as Euler angles in degrees)
                    ui.label("Rotation:");
                    let (x, y, z) = self.rotation.to_euler(glam::EulerRot::XYZ);
                    ui.label(format!("{:.1}° {:.1}° {:.1}°", 
                        x.to_degrees(), y.to_degrees(), z.to_degrees()));
                    ui.end_row();

                    // Scale row
                    ui.label("Scale:");
                    ui.label(format!("{:.2} {:.2} {:.2}", 
                        self.scale.x, self.scale.y, self.scale.z));
                    ui.end_row();
                });

            ui.add_space(8.0);
        }

        /// Render a compact single-line version
        pub fn render_egui_compact(&self, ui: &mut Ui) {
            let header_color = if self.is_global {
                Color32::from_rgb(100, 150, 255)
            } else {
                Color32::from_rgb(150, 255, 100)
            };

            ui.horizontal(|ui| {
                ui.colored_label(header_color, self.get_type_label());
                ui.separator();
                ui.label(format!("T: {:.1} {:.1} {:.1}", 
                    self.translation.x, self.translation.y, self.translation.z));
                ui.separator();
                let (x, y, z) = self.rotation.to_euler(glam::EulerRot::XYZ);
                ui.label(format!("R: {:.0}° {:.0}° {:.0}°", 
                    x.to_degrees(), y.to_degrees(), z.to_degrees()));
                ui.separator();
                ui.label(format!("S: {:.1} {:.1} {:.1}", 
                    self.scale.x, self.scale.y, self.scale.z));
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::{Vec3, Quat};

    #[test]
    fn test_transform_display_formatting() {
        let transform = TransformDisplay::new_local(
            Vec3::new(1.234, 2.567, 3.891),
            Quat::from_rotation_y(std::f32::consts::PI / 4.0),
            Vec3::new(1.0, 1.5, 2.0),
        );

        assert_eq!(transform.format_translation(), "Translation: 1.23 2.57 3.89");
        assert_eq!(transform.get_type_label(), "Local Transform");
        
        let lines = transform.format_aligned();
        assert_eq!(lines.len(), 5);
        assert!(lines[0].contains("Local Transform"));
    }

    #[test]
    fn test_global_transform_display() {
        let transform = TransformDisplay::new_global(
            Vec3::ZERO,
            Quat::IDENTITY,
            Vec3::ONE,
        );

        assert_eq!(transform.get_type_label(), "Global Transform");
        assert_eq!(transform.format_scale(), "Scale: 1.00 1.00 1.00");
    }
}
