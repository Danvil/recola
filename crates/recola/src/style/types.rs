//! Core style types and components for the hierarchical style system

use atom::prelude::*;
use glam::{Vec2, Vec4};
use std::sync::Arc;

/// Comprehensive style definition with all fields as non-Option types.
/// This represents the computed/inherited style for an entity.
#[derive(Debug, Clone, PartialEq)]
pub struct Style {
    // Layout properties
    pub margin: Vec4,           // top, right, bottom, left
    pub padding: Vec4,          // top, right, bottom, left
    pub size: Vec2,             // width, height
    pub min_size: Vec2,         // minimum width, height
    pub max_size: Vec2,         // maximum width, height
    
    // Visual properties
    pub background_color: Vec4,  // RGBA
    pub foreground_color: Vec4,  // RGBA (text color)
    pub border_color: Vec4,      // RGBA
    pub border_width: f32,
    pub border_radius: f32,
    
    // Typography
    pub font_size: f32,
    pub line_height: f32,
    pub font_weight: FontWeight,
    pub text_align: TextAlign,
    
    // Effects
    pub opacity: f32,
    pub shadow_offset: Vec2,
    pub shadow_blur: f32,
    pub shadow_color: Vec4,     // RGBA
    
    // Interaction
    pub cursor_style: CursorStyle,
    pub interactive: bool,
    
    // Animation
    pub transition_duration: f32,
    pub animation_curve: AnimationCurve,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            // Layout defaults
            margin: Vec4::ZERO,
            padding: Vec4::ZERO,
            size: Vec2::new(100.0, 100.0),
            min_size: Vec2::ZERO,
            max_size: Vec2::new(f32::INFINITY, f32::INFINITY),
            
            // Visual defaults
            background_color: Vec4::new(1.0, 1.0, 1.0, 1.0), // white
            foreground_color: Vec4::new(0.0, 0.0, 0.0, 1.0), // black
            border_color: Vec4::new(0.5, 0.5, 0.5, 1.0),     // gray
            border_width: 0.0,
            border_radius: 0.0,
            
            // Typography defaults
            font_size: 14.0,
            line_height: 1.2,
            font_weight: FontWeight::Normal,
            text_align: TextAlign::Left,
            
            // Effects defaults
            opacity: 1.0,
            shadow_offset: Vec2::ZERO,
            shadow_blur: 0.0,
            shadow_color: Vec4::new(0.0, 0.0, 0.0, 0.3), // semi-transparent black
            
            // Interaction defaults
            cursor_style: CursorStyle::Default,
            interactive: false,
            
            // Animation defaults
            transition_duration: 0.0,
            animation_curve: AnimationCurve::Linear,
        }
    }
}

/// Font weight enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontWeight {
    Thin,
    Light,
    Normal,
    Medium,
    Bold,
    Black,
}

/// Text alignment options
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextAlign {
    Left,
    Center,
    Right,
    Justify,
}

/// Cursor style options
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CursorStyle {
    Default,
    Pointer,
    Text,
    Crosshair,
    Move,
    NotAllowed,
    Grab,
    Grabbing,
}

/// Animation curve types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimationCurve {
    Linear,
    EaseIn,
    EaseOut,
    EaseInOut,
    Bounce,
}

/// Component that holds a shared reference to a computed style.
/// Uses Arc for memory efficiency when multiple entities share the same style.
#[derive(Component, Debug, Clone)]
pub struct SharedStyle {
    pub style: Arc<Style>,
}

impl SharedStyle {
    pub fn new(style: Style) -> Self {
        Self {
            style: Arc::new(style),
        }
    }
    
    pub fn from_arc(style: Arc<Style>) -> Self {
        Self { style }
    }
}

/// Marker component indicating that an entity's style needs to be recomputed.
/// When an entity is marked as StyleDirty, its descendants will also be updated.
#[derive(Component, Debug, Default)]
pub struct StyleDirty;

/// Optional border style component that can be applied to entities.
/// This will be merged into the computed Style during hierarchical processing.
#[derive(Component, Debug, Clone)]
pub struct BorderStyle {
    pub color: Option<Vec4>,
    pub width: Option<f32>,
    pub radius: Option<f32>,
}

impl Default for BorderStyle {
    fn default() -> Self {
        Self {
            color: None,
            width: None,
            radius: None,
        }
    }
}

/// Optional panel style component that can be applied to entities.
/// This will be merged into the computed Style during hierarchical processing.
#[derive(Component, Debug, Clone)]
pub struct PanelStyle {
    pub background_color: Option<Vec4>,
    pub padding: Option<Vec4>,
    pub margin: Option<Vec4>,
    pub opacity: Option<f32>,
}

impl Default for PanelStyle {
    fn default() -> Self {
        Self {
            background_color: None,
            padding: None,
            margin: None,
            opacity: None,
        }
    }
}

/// Optional text style component that can be applied to entities.
/// This will be merged into the computed Style during hierarchical processing.
#[derive(Component, Debug, Clone)]
pub struct TextStyle {
    pub color: Option<Vec4>,
    pub font_size: Option<f32>,
    pub font_weight: Option<FontWeight>,
    pub text_align: Option<TextAlign>,
    pub line_height: Option<f32>,
}

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            color: None,
            font_size: None,
            font_weight: None,
            text_align: None,
            line_height: None,
        }
    }
}

/// Optional layout style component that can be applied to entities.
/// This will be merged into the computed Style during hierarchical processing.
#[derive(Component, Debug, Clone)]
pub struct LayoutStyle {
    pub size: Option<Vec2>,
    pub min_size: Option<Vec2>,
    pub max_size: Option<Vec2>,
    pub margin: Option<Vec4>,
    pub padding: Option<Vec4>,
}

impl Default for LayoutStyle {
    fn default() -> Self {
        Self {
            size: None,
            min_size: None,
            max_size: None,
            margin: None,
            padding: None,
        }
    }
}
