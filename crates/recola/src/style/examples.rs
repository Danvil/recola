//! Example usage of the hierarchical style system
//! 
//! This module demonstrates how to create styled entities and work with
//! the hierarchical style inheritance system.

use super::{mocca::helpers::*, types::*, StyledEntityCommands};
use atom::prelude::*;
use candy::scene_tree::*;
use glam::{Vec2, Vec4};

/// Example function showing how to create a styled UI hierarchy
/// This would typically be called from a system or during entity spawning
pub fn create_styled_ui_example(mut cmd: Commands) {
    // Create a root panel with a dark background and padding
    let root_panel = cmd
        .spawn((
            Name::from_str("root_panel"),
            Transform3::identity(),
        ))
        .with_panel_style(panel(colors::DARK_GRAY, 20.0))
        .with_text_style(text(colors::WHITE, 16.0))
        .id();

    // Create a child button with a border and different background
    let button = cmd
        .spawn((
            Name::from_str("button"),
            Transform3::identity(),
            (ChildOf, root_panel),
        ))
        .with_panel_style(panel(colors::BLUE, 10.0))
        .with_border_style(rounded_border(colors::LIGHT_GRAY, 2.0, 5.0))
        .with_layout_style(layout_size(120.0, 40.0))
        .id();

    // Create button text that inherits the white color from root but has smaller font
    let _button_text = cmd
        .spawn((
            Name::from_str("button_text"),
            Transform3::identity(),
            (ChildOf, button),
        ))
        .with_text_style(text(colors::WHITE, 14.0))
        .id();

    // Create a sidebar with different styling
    let sidebar = cmd
        .spawn((
            Name::from_str("sidebar"),
            Transform3::identity(),
            (ChildOf, root_panel),
        ))
        .with_panel_style(PanelStyle {
            background_color: Some(colors::GRAY),
            padding: Some(Vec4::new(15.0, 15.0, 15.0, 15.0)),
            margin: Some(Vec4::new(10.0, 0.0, 10.0, 0.0)),
            opacity: Some(0.9),
        })
        .with_layout_style(layout_size(200.0, 300.0))
        .id();

    // Create sidebar items that inherit the sidebar's styling
    for i in 0..3 {
        let _sidebar_item = cmd
            .spawn((
                Name::from_str(&format!("sidebar_item_{}", i)),
                Transform3::identity(),
                (ChildOf, sidebar),
            ))
            .with_panel_style(PanelStyle {
                background_color: Some(colors::LIGHT_GRAY),
                padding: Some(Vec4::splat(8.0)),
                margin: Some(Vec4::new(0.0, 0.0, 5.0, 0.0)), // bottom margin
                opacity: None, // inherit from parent
            })
            .with_text_style(TextStyle {
                color: Some(colors::BLACK),
                font_size: Some(12.0),
                ..Default::default()
            })
            .id();
    }
}

/// Example showing how to dynamically update styles
pub fn update_style_example(
    mut cmd: Commands,
    query_buttons: Query<Entity, (With<Name>, With<BorderStyle>)>,
) {
    // Find all button entities and update their border color
    for entity in query_buttons.iter() {
        // Update the border style - this will automatically mark the entity as StyleDirty
        cmd.entity(entity).and_set(BorderStyle {
            color: Some(colors::RED), // Change border to red
            width: Some(3.0),         // Make border thicker
            radius: Some(8.0),        // More rounded corners
        });
    }
}

/// Example showing how to create a themed component
pub fn create_themed_card(cmd: &mut Commands, parent: Entity, title: &str, content: &str) -> Entity {
    // Create the card container
    let card = cmd
        .spawn((
            Name::from_str(&format!("card_{}", title)),
            Transform3::identity(),
            (ChildOf, parent),
        ))
        .with_panel_style(PanelStyle {
            background_color: Some(colors::WHITE),
            padding: Some(Vec4::splat(16.0)),
            margin: Some(Vec4::new(8.0, 8.0, 8.0, 8.0)),
            opacity: Some(1.0),
        })
        .with_border_style(BorderStyle {
            color: Some(colors::LIGHT_GRAY),
            width: Some(1.0),
            radius: Some(8.0),
        })
        .with_layout_style(LayoutStyle {
            size: Some(Vec2::new(250.0, 150.0)),
            min_size: Some(Vec2::new(200.0, 100.0)),
            max_size: Some(Vec2::new(300.0, 200.0)),
            ..Default::default()
        })
        .id();

    // Create the card title
    let _title = cmd
        .spawn((
            Name::from_str(&format!("card_title_{}", title)),
            Transform3::identity(),
            (ChildOf, card),
        ))
        .with_text_style(TextStyle {
            color: Some(colors::DARK_GRAY),
            font_size: Some(18.0),
            font_weight: Some(FontWeight::Bold),
            ..Default::default()
        })
        .with_layout_style(LayoutStyle {
            margin: Some(Vec4::new(0.0, 0.0, 12.0, 0.0)), // bottom margin
            ..Default::default()
        })
        .id();

    // Create the card content
    let _content = cmd
        .spawn((
            Name::from_str(&format!("card_content_{}", title)),
            Transform3::identity(),
            (ChildOf, card),
        ))
        .with_text_style(TextStyle {
            color: Some(colors::BLACK),
            font_size: Some(14.0),
            line_height: Some(1.4),
            ..Default::default()
        })
        .id();

    card
}

/// Example showing how to create a style theme
pub struct Theme {
    pub primary_color: Vec4,
    pub secondary_color: Vec4,
    pub background_color: Vec4,
    pub text_color: Vec4,
    pub border_color: Vec4,
    pub font_size_base: f32,
    pub font_size_large: f32,
    pub font_size_small: f32,
    pub border_radius: f32,
    pub padding_base: f32,
}

impl Theme {
    pub fn dark_theme() -> Self {
        Self {
            primary_color: Vec4::new(0.2, 0.6, 1.0, 1.0),    // blue
            secondary_color: Vec4::new(0.8, 0.4, 1.0, 1.0),  // purple
            background_color: Vec4::new(0.1, 0.1, 0.1, 1.0), // dark gray
            text_color: Vec4::new(0.9, 0.9, 0.9, 1.0),       // light gray
            border_color: Vec4::new(0.3, 0.3, 0.3, 1.0),     // medium gray
            font_size_base: 14.0,
            font_size_large: 18.0,
            font_size_small: 12.0,
            border_radius: 6.0,
            padding_base: 12.0,
        }
    }

    pub fn light_theme() -> Self {
        Self {
            primary_color: Vec4::new(0.0, 0.4, 0.8, 1.0),    // darker blue
            secondary_color: Vec4::new(0.6, 0.2, 0.8, 1.0),  // darker purple
            background_color: Vec4::new(0.95, 0.95, 0.95, 1.0), // light gray
            text_color: Vec4::new(0.1, 0.1, 0.1, 1.0),       // dark gray
            border_color: Vec4::new(0.7, 0.7, 0.7, 1.0),     // medium gray
            font_size_base: 14.0,
            font_size_large: 18.0,
            font_size_small: 12.0,
            border_radius: 4.0,
            padding_base: 10.0,
        }
    }

    /// Create a primary button style using this theme
    pub fn primary_button_style(&self) -> (PanelStyle, BorderStyle, TextStyle) {
        let panel = PanelStyle {
            background_color: Some(self.primary_color),
            padding: Some(Vec4::splat(self.padding_base)),
            opacity: Some(1.0),
            margin: None,
        };

        let border = BorderStyle {
            color: Some(self.primary_color),
            width: Some(2.0),
            radius: Some(self.border_radius),
        };

        let text = TextStyle {
            color: Some(colors::WHITE),
            font_size: Some(self.font_size_base),
            font_weight: Some(FontWeight::Medium),
            text_align: Some(TextAlign::Center),
            line_height: None,
        };

        (panel, border, text)
    }

    /// Create a secondary button style using this theme
    pub fn secondary_button_style(&self) -> (PanelStyle, BorderStyle, TextStyle) {
        let panel = PanelStyle {
            background_color: Some(self.background_color),
            padding: Some(Vec4::splat(self.padding_base)),
            opacity: Some(1.0),
            margin: None,
        };

        let border = BorderStyle {
            color: Some(self.primary_color),
            width: Some(2.0),
            radius: Some(self.border_radius),
        };

        let text = TextStyle {
            color: Some(self.primary_color),
            font_size: Some(self.font_size_base),
            font_weight: Some(FontWeight::Medium),
            text_align: Some(TextAlign::Center),
            line_height: None,
        };

        (panel, border, text)
    }
}

/// Example showing how to apply a theme to create consistent UI components
pub fn create_themed_button(
    cmd: &mut Commands,
    parent: Entity,
    theme: &Theme,
    label: &str,
    is_primary: bool,
) -> Entity {
    let (panel_style, border_style, text_style) = if is_primary {
        theme.primary_button_style()
    } else {
        theme.secondary_button_style()
    };

    let button = cmd
        .spawn((
            Name::from_str(&format!("themed_button_{}", label)),
            Transform3::identity(),
            (ChildOf, parent),
        ))
        .with_panel_style(panel_style)
        .with_border_style(border_style)
        .with_layout_style(layout_size(100.0, 36.0))
        .id();

    let _button_text = cmd
        .spawn((
            Name::from_str(&format!("themed_button_text_{}", label)),
            Transform3::identity(),
            (ChildOf, button),
        ))
        .with_text_style(text_style)
        .id();

    button
}
