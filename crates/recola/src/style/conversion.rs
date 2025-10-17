//! Conversion utilities for merging optional style components into the unified Style type

use super::types::*;

/// Trait for merging optional style components into a base Style
pub trait MergeIntoStyle {
    fn merge_into(&self, style: &mut Style);
}

impl MergeIntoStyle for BorderStyle {
    fn merge_into(&self, style: &mut Style) {
        if let Some(color) = self.color {
            style.border_color = color;
        }
        if let Some(width) = self.width {
            style.border_width = width;
        }
        if let Some(radius) = self.radius {
            style.border_radius = radius;
        }
    }
}

impl MergeIntoStyle for PanelStyle {
    fn merge_into(&self, style: &mut Style) {
        if let Some(background_color) = self.background_color {
            style.background_color = background_color;
        }
        if let Some(padding) = self.padding {
            style.padding = padding;
        }
        if let Some(margin) = self.margin {
            style.margin = margin;
        }
        if let Some(opacity) = self.opacity {
            style.opacity = opacity;
        }
    }
}

impl MergeIntoStyle for TextStyle {
    fn merge_into(&self, style: &mut Style) {
        if let Some(color) = self.color {
            style.foreground_color = color;
        }
        if let Some(font_size) = self.font_size {
            style.font_size = font_size;
        }
        if let Some(font_weight) = self.font_weight {
            style.font_weight = font_weight;
        }
        if let Some(text_align) = self.text_align {
            style.text_align = text_align;
        }
        if let Some(line_height) = self.line_height {
            style.line_height = line_height;
        }
    }
}

impl MergeIntoStyle for LayoutStyle {
    fn merge_into(&self, style: &mut Style) {
        if let Some(size) = self.size {
            style.size = size;
        }
        if let Some(min_size) = self.min_size {
            style.min_size = min_size;
        }
        if let Some(max_size) = self.max_size {
            style.max_size = max_size;
        }
        if let Some(margin) = self.margin {
            style.margin = margin;
        }
        if let Some(padding) = self.padding {
            style.padding = padding;
        }
    }
}

/// Helper function to create a Style by merging multiple optional style components
pub fn merge_styles(
    base: Style,
    border: Option<&BorderStyle>,
    panel: Option<&PanelStyle>,
    text: Option<&TextStyle>,
    layout: Option<&LayoutStyle>,
) -> Style {
    let mut style = base;
    
    if let Some(border_style) = border {
        border_style.merge_into(&mut style);
    }
    
    if let Some(panel_style) = panel {
        panel_style.merge_into(&mut style);
    }
    
    if let Some(text_style) = text {
        text_style.merge_into(&mut style);
    }
    
    if let Some(layout_style) = layout {
        layout_style.merge_into(&mut style);
    }
    
    style
}

/// Helper function to inherit styles from parent to child
/// Child styles override parent styles where specified
pub fn inherit_style(parent_style: &Style, child_overrides: &Style) -> Style {
    // For now, we do a simple field-by-field override
    // In a more sophisticated system, you might want different inheritance rules
    // for different properties (e.g., some properties might multiply, others replace)
    Style {
        // Layout properties - child overrides parent
        margin: child_overrides.margin,
        padding: child_overrides.padding,
        size: child_overrides.size,
        min_size: child_overrides.min_size,
        max_size: child_overrides.max_size,
        
        // Visual properties - child overrides parent
        background_color: child_overrides.background_color,
        foreground_color: child_overrides.foreground_color,
        border_color: child_overrides.border_color,
        border_width: child_overrides.border_width,
        border_radius: child_overrides.border_radius,
        
        // Typography - child overrides parent, but inherits if child is default
        font_size: if child_overrides.font_size != Style::default().font_size {
            child_overrides.font_size
        } else {
            parent_style.font_size
        },
        line_height: if child_overrides.line_height != Style::default().line_height {
            child_overrides.line_height
        } else {
            parent_style.line_height
        },
        font_weight: if child_overrides.font_weight != Style::default().font_weight {
            child_overrides.font_weight
        } else {
            parent_style.font_weight
        },
        text_align: if child_overrides.text_align != Style::default().text_align {
            child_overrides.text_align
        } else {
            parent_style.text_align
        },
        
        // Effects - multiply opacity, override others
        opacity: parent_style.opacity * child_overrides.opacity,
        shadow_offset: child_overrides.shadow_offset,
        shadow_blur: child_overrides.shadow_blur,
        shadow_color: child_overrides.shadow_color,
        
        // Interaction - child overrides parent
        cursor_style: child_overrides.cursor_style,
        interactive: child_overrides.interactive,
        
        // Animation - child overrides parent
        transition_duration: child_overrides.transition_duration,
        animation_curve: child_overrides.animation_curve,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::{Vec2, Vec4};

    #[test]
    fn test_border_style_merge() {
        let mut style = Style::default();
        let border_style = BorderStyle {
            color: Some(Vec4::new(1.0, 0.0, 0.0, 1.0)), // red
            width: Some(2.0),
            radius: Some(5.0),
        };
        
        border_style.merge_into(&mut style);
        
        assert_eq!(style.border_color, Vec4::new(1.0, 0.0, 0.0, 1.0));
        assert_eq!(style.border_width, 2.0);
        assert_eq!(style.border_radius, 5.0);
    }

    #[test]
    fn test_panel_style_merge() {
        let mut style = Style::default();
        let panel_style = PanelStyle {
            background_color: Some(Vec4::new(0.0, 1.0, 0.0, 1.0)), // green
            padding: Some(Vec4::new(10.0, 10.0, 10.0, 10.0)),
            margin: None,
            opacity: Some(0.8),
        };
        
        panel_style.merge_into(&mut style);
        
        assert_eq!(style.background_color, Vec4::new(0.0, 1.0, 0.0, 1.0));
        assert_eq!(style.padding, Vec4::new(10.0, 10.0, 10.0, 10.0));
        assert_eq!(style.opacity, 0.8);
        // margin should remain default since it was None
        assert_eq!(style.margin, Vec4::ZERO);
    }

    #[test]
    fn test_style_inheritance() {
        let parent_style = Style {
            font_size: 16.0,
            foreground_color: Vec4::new(0.2, 0.2, 0.2, 1.0),
            opacity: 0.9,
            ..Style::default()
        };
        
        let child_overrides = Style {
            font_size: 12.0, // override parent
            background_color: Vec4::new(1.0, 1.0, 0.0, 1.0), // yellow
            opacity: 0.8, // will be multiplied with parent
            ..Style::default()
        };
        
        let inherited = inherit_style(&parent_style, &child_overrides);
        
        assert_eq!(inherited.font_size, 12.0); // child override
        assert_eq!(inherited.foreground_color, Vec4::new(0.2, 0.2, 0.2, 1.0)); // inherited from parent
        assert_eq!(inherited.background_color, Vec4::new(1.0, 1.0, 0.0, 1.0)); // child override
        assert_eq!(inherited.opacity, 0.9 * 0.8); // multiplied
    }
}
