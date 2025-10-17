//! Mocca system integration for the hierarchical style system

use super::{hierarchy::*, types::*};
use atom::prelude::*;
use candy::scene_tree::CandySceneTreeMocca;

/// Mocca implementation for the hierarchical style system.
/// This integrates the style system into the application's ECS framework.
pub struct StyleMocca;

impl Mocca for StyleMocca {
    fn load(mut deps: MoccaDeps) {
        // Depend on the scene tree system for hierarchical processing
        deps.depends_on::<CandySceneTreeMocca>();
    }

    fn start(_world: &mut World) -> Self {
        Self
    }

    fn register_components(world: &mut World) {
        // Register all style-related components
        world.register_component::<SharedStyle>();
        world.register_component::<StyleDirty>();
        world.register_component::<BorderStyle>();
        world.register_component::<PanelStyle>();
        world.register_component::<TextStyle>();
        world.register_component::<LayoutStyle>();
    }

    fn step(&mut self, world: &mut World) {
        // Run style systems in the correct order
        
        // 1. First, mark entities as dirty when their style components change
        world.run(mark_style_dirty_on_component_changes);
        
        // 2. Propagate dirty flags to children when parent styles change
        world.run(propagate_style_dirty_to_children);
        
        // 3. Finally, update the hierarchical styles for all dirty entities
        world.run(update_hierarchical_styles);
    }
}

/// Helper trait to make it easier to work with styled entities
pub trait StyledEntityCommands {
    /// Add a border style to an entity and mark it as dirty
    fn with_border_style(self, border_style: BorderStyle) -> Self;
    
    /// Add a panel style to an entity and mark it as dirty
    fn with_panel_style(self, panel_style: PanelStyle) -> Self;
    
    /// Add a text style to an entity and mark it as dirty
    fn with_text_style(self, text_style: TextStyle) -> Self;
    
    /// Add a layout style to an entity and mark it as dirty
    fn with_layout_style(self, layout_style: LayoutStyle) -> Self;
    
    /// Mark an entity's style as dirty to trigger recomputation
    fn mark_style_dirty(self) -> Self;
}

impl StyledEntityCommands for EntityCommands<'_> {
    fn with_border_style(self, border_style: BorderStyle) -> Self {
        self.and_set(border_style).and_set(StyleDirty)
    }
    
    fn with_panel_style(self, panel_style: PanelStyle) -> Self {
        self.and_set(panel_style).and_set(StyleDirty)
    }
    
    fn with_text_style(self, text_style: TextStyle) -> Self {
        self.and_set(text_style).and_set(StyleDirty)
    }
    
    fn with_layout_style(self, layout_style: LayoutStyle) -> Self {
        self.and_set(layout_style).and_set(StyleDirty)
    }
    
    fn mark_style_dirty(self) -> Self {
        self.and_set(StyleDirty)
    }
}

/// Helper functions for common styling operations
pub mod helpers {
    use super::*;
    use glam::{Vec2, Vec4};

    /// Create a simple border style with color and width
    pub fn border(color: Vec4, width: f32) -> BorderStyle {
        BorderStyle {
            color: Some(color),
            width: Some(width),
            radius: None,
        }
    }

    /// Create a rounded border style with color, width, and radius
    pub fn rounded_border(color: Vec4, width: f32, radius: f32) -> BorderStyle {
        BorderStyle {
            color: Some(color),
            width: Some(width),
            radius: Some(radius),
        }
    }

    /// Create a panel style with background color and padding
    pub fn panel(background_color: Vec4, padding: f32) -> PanelStyle {
        PanelStyle {
            background_color: Some(background_color),
            padding: Some(Vec4::splat(padding)),
            margin: None,
            opacity: None,
        }
    }

    /// Create a text style with color and font size
    pub fn text(color: Vec4, font_size: f32) -> TextStyle {
        TextStyle {
            color: Some(color),
            font_size: Some(font_size),
            font_weight: None,
            text_align: None,
            line_height: None,
        }
    }

    /// Create a layout style with size
    pub fn layout_size(width: f32, height: f32) -> LayoutStyle {
        LayoutStyle {
            size: Some(Vec2::new(width, height)),
            min_size: None,
            max_size: None,
            margin: None,
            padding: None,
        }
    }

    /// Create a layout style with padding
    pub fn layout_padding(padding: f32) -> LayoutStyle {
        LayoutStyle {
            size: None,
            min_size: None,
            max_size: None,
            margin: None,
            padding: Some(Vec4::splat(padding)),
        }
    }

    /// Create a layout style with margin
    pub fn layout_margin(margin: f32) -> LayoutStyle {
        LayoutStyle {
            size: None,
            min_size: None,
            max_size: None,
            margin: Some(Vec4::splat(margin)),
            padding: None,
        }
    }

    /// Common color constants for convenience
    pub mod colors {
        use glam::Vec4;

        pub const WHITE: Vec4 = Vec4::new(1.0, 1.0, 1.0, 1.0);
        pub const BLACK: Vec4 = Vec4::new(0.0, 0.0, 0.0, 1.0);
        pub const RED: Vec4 = Vec4::new(1.0, 0.0, 0.0, 1.0);
        pub const GREEN: Vec4 = Vec4::new(0.0, 1.0, 0.0, 1.0);
        pub const BLUE: Vec4 = Vec4::new(0.0, 0.0, 1.0, 1.0);
        pub const YELLOW: Vec4 = Vec4::new(1.0, 1.0, 0.0, 1.0);
        pub const CYAN: Vec4 = Vec4::new(0.0, 1.0, 1.0, 1.0);
        pub const MAGENTA: Vec4 = Vec4::new(1.0, 0.0, 1.0, 1.0);
        pub const GRAY: Vec4 = Vec4::new(0.5, 0.5, 0.5, 1.0);
        pub const LIGHT_GRAY: Vec4 = Vec4::new(0.8, 0.8, 0.8, 1.0);
        pub const DARK_GRAY: Vec4 = Vec4::new(0.2, 0.2, 0.2, 1.0);
        pub const TRANSPARENT: Vec4 = Vec4::new(0.0, 0.0, 0.0, 0.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::helpers::*;

    #[test]
    fn test_border_helper() {
        let border_style = border(helpers::colors::RED, 2.0);
        assert_eq!(border_style.color, Some(helpers::colors::RED));
        assert_eq!(border_style.width, Some(2.0));
        assert_eq!(border_style.radius, None);
    }

    #[test]
    fn test_rounded_border_helper() {
        let border_style = rounded_border(helpers::colors::BLUE, 1.0, 5.0);
        assert_eq!(border_style.color, Some(helpers::colors::BLUE));
        assert_eq!(border_style.width, Some(1.0));
        assert_eq!(border_style.radius, Some(5.0));
    }

    #[test]
    fn test_panel_helper() {
        let panel_style = panel(helpers::colors::LIGHT_GRAY, 10.0);
        assert_eq!(panel_style.background_color, Some(helpers::colors::LIGHT_GRAY));
        assert_eq!(panel_style.padding, Some(Vec4::splat(10.0)));
        assert_eq!(panel_style.margin, None);
        assert_eq!(panel_style.opacity, None);
    }

    #[test]
    fn test_text_helper() {
        let text_style = text(helpers::colors::BLACK, 16.0);
        assert_eq!(text_style.color, Some(helpers::colors::BLACK));
        assert_eq!(text_style.font_size, Some(16.0));
        assert_eq!(text_style.font_weight, None);
    }

    #[test]
    fn test_layout_size_helper() {
        let layout_style = layout_size(100.0, 50.0);
        assert_eq!(layout_style.size, Some(Vec2::new(100.0, 50.0)));
        assert_eq!(layout_style.min_size, None);
        assert_eq!(layout_style.max_size, None);
    }
}
