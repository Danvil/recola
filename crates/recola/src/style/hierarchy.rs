//! Hierarchical style processing system
//! 
//! This module implements the core logic for computing inherited styles from parent to child entities,
//! similar to the visibility system in candy_scene_tree.

use super::{conversion::*, types::*};
use atom::prelude::*;
use candy::scene_tree::*;
use std::{collections::HashSet, sync::Arc};

/// System that processes entities marked with StyleDirty and computes their inherited styles.
/// This system traverses the entity hierarchy and propagates style changes to descendants.
pub fn update_hierarchical_styles(
    mut cmd: Commands,
    children: Relation<ChildOf>,
    mut query_dirty: Query<Entity, With<StyleDirty>>,
    query_shared_style: Query<&SharedStyle>,
    query_border_style: Query<&BorderStyle>,
    query_panel_style: Query<&PanelStyle>,
    query_text_style: Query<&TextStyle>,
    query_layout_style: Query<&LayoutStyle>,
) {
    // Collect all dirty entities
    let dirty_entities: HashSet<Entity> = query_dirty.iter().collect();
    
    if dirty_entities.is_empty() {
        return;
    }
    
    // Process each dirty entity and its descendants
    for entity in dirty_entities.iter().copied() {
        process_entity_style_hierarchy(
            entity,
            &mut cmd,
            &children,
            &query_shared_style,
            &query_border_style,
            &query_panel_style,
            &query_text_style,
            &query_layout_style,
            None, // root entity has no parent style
        );
        
        // Remove the StyleDirty marker from the processed entity
        cmd.entity(entity).remove::<StyleDirty>();
    }
}

/// Recursively processes an entity and its descendants to compute inherited styles
fn process_entity_style_hierarchy(
    entity: Entity,
    cmd: &mut Commands,
    children: &Relation<ChildOf>,
    query_shared_style: &Query<&SharedStyle>,
    query_border_style: &Query<&BorderStyle>,
    query_panel_style: &Query<&PanelStyle>,
    query_text_style: &Query<&TextStyle>,
    query_layout_style: &Query<&LayoutStyle>,
    parent_style: Option<&Style>,
) {
    // Compute the style for this entity
    let computed_style = compute_entity_style(
        entity,
        query_border_style,
        query_panel_style,
        query_text_style,
        query_layout_style,
        parent_style,
    );
    
    // Check if we need to update the SharedStyle component
    let needs_update = match query_shared_style.get(entity) {
        Ok(existing_shared_style) => *existing_shared_style.style != computed_style,
        Err(_) => true, // Entity doesn't have SharedStyle yet
    };
    
    if needs_update {
        // Update or add the SharedStyle component
        cmd.entity(entity).and_set(SharedStyle::new(computed_style.clone()));
    }
    
    // Process all children recursively
    for child_entity in children.iter(entity) {
        process_entity_style_hierarchy(
            child_entity,
            cmd,
            children,
            query_shared_style,
            query_border_style,
            query_panel_style,
            query_text_style,
            query_layout_style,
            Some(&computed_style),
        );
    }
}

/// Computes the final style for an entity by merging its style components with inherited parent style
fn compute_entity_style(
    entity: Entity,
    query_border_style: &Query<&BorderStyle>,
    query_panel_style: &Query<&PanelStyle>,
    query_text_style: &Query<&TextStyle>,
    query_layout_style: &Query<&LayoutStyle>,
    parent_style: Option<&Style>,
) -> Style {
    // Start with default style or inherited parent style
    let base_style = parent_style.cloned().unwrap_or_default();
    
    // Collect optional style components for this entity
    let border_style = query_border_style.get(entity).ok();
    let panel_style = query_panel_style.get(entity).ok();
    let text_style = query_text_style.get(entity).ok();
    let layout_style = query_layout_style.get(entity).ok();
    
    // Merge all style components into the base style
    merge_styles(
        base_style,
        border_style,
        panel_style,
        text_style,
        layout_style,
    )
}

/// System that automatically marks entities as StyleDirty when their style components change
pub fn mark_style_dirty_on_component_changes(
    mut cmd: Commands,
    // Track entities that have had their style components added or changed
    query_border_changed: Query<Entity, (With<BorderStyle>, Changed<BorderStyle>)>,
    query_panel_changed: Query<Entity, (With<PanelStyle>, Changed<PanelStyle>)>,
    query_text_changed: Query<Entity, (With<TextStyle>, Changed<TextStyle>)>,
    query_layout_changed: Query<Entity, (With<LayoutStyle>, Changed<LayoutStyle>)>,
) {
    // Mark entities as dirty when their style components change
    for entity in query_border_changed.iter() {
        cmd.entity(entity).and_set(StyleDirty);
    }
    
    for entity in query_panel_changed.iter() {
        cmd.entity(entity).and_set(StyleDirty);
    }
    
    for entity in query_text_changed.iter() {
        cmd.entity(entity).and_set(StyleDirty);
    }
    
    for entity in query_layout_changed.iter() {
        cmd.entity(entity).and_set(StyleDirty);
    }
}

/// System that marks child entities as StyleDirty when their parent's style changes
pub fn propagate_style_dirty_to_children(
    mut cmd: Commands,
    children: Relation<ChildOf>,
    query_dirty_parents: Query<Entity, (With<StyleDirty>, With<SharedStyle>)>,
) {
    for parent_entity in query_dirty_parents.iter() {
        // Mark all children as dirty when parent style changes
        mark_descendants_dirty(parent_entity, &mut cmd, &children);
    }
}

/// Recursively marks all descendants of an entity as StyleDirty
fn mark_descendants_dirty(
    entity: Entity,
    cmd: &mut Commands,
    children: &Relation<ChildOf>,
) {
    for child_entity in children.iter(entity) {
        cmd.entity(child_entity).and_set(StyleDirty);
        // Recursively mark grandchildren and beyond
        mark_descendants_dirty(child_entity, cmd, children);
    }
}

/// Helper function to manually trigger style recomputation for an entity and its descendants
pub fn mark_entity_style_dirty(cmd: &mut Commands, entity: Entity) {
    cmd.entity(entity).and_set(StyleDirty);
}

/// Helper function to get the computed style for an entity
pub fn get_entity_style(entity: Entity, query_shared_style: &Query<&SharedStyle>) -> Option<Arc<Style>> {
    query_shared_style.get(entity).ok().map(|shared| shared.style.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::{Vec2, Vec4};

    #[test]
    fn test_compute_entity_style_with_no_components() {
        // Create mock queries (this is a simplified test)
        // In a real test, you'd need to set up a proper ECS world
        let entity = Entity::from_raw(0);
        
        // Mock empty queries
        let empty_border_query = Query::<&BorderStyle>::new();
        let empty_panel_query = Query::<&PanelStyle>::new();
        let empty_text_query = Query::<&TextStyle>::new();
        let empty_layout_query = Query::<&LayoutStyle>::new();
        
        let style = compute_entity_style(
            entity,
            &empty_border_query,
            &empty_panel_query,
            &empty_text_query,
            &empty_layout_query,
            None,
        );
        
        // Should return default style when no components are present
        assert_eq!(style, Style::default());
    }
    
    #[test]
    fn test_style_inheritance_logic() {
        let parent_style = Style {
            font_size: 18.0,
            foreground_color: Vec4::new(0.1, 0.1, 0.1, 1.0),
            background_color: Vec4::new(0.9, 0.9, 0.9, 1.0),
            ..Style::default()
        };
        
        // Test that parent style is used as base when no local components exist
        let inherited = merge_styles(parent_style.clone(), None, None, None, None);
        assert_eq!(inherited.font_size, 18.0);
        assert_eq!(inherited.foreground_color, Vec4::new(0.1, 0.1, 0.1, 1.0));
        
        // Test that local components override parent style
        let text_style = TextStyle {
            font_size: Some(14.0),
            color: Some(Vec4::new(1.0, 0.0, 0.0, 1.0)), // red
            ..Default::default()
        };
        
        let overridden = merge_styles(parent_style, None, None, Some(&text_style), None);
        assert_eq!(overridden.font_size, 14.0); // overridden
        assert_eq!(overridden.foreground_color, Vec4::new(1.0, 0.0, 0.0, 1.0)); // overridden
        assert_eq!(overridden.background_color, Vec4::new(0.9, 0.9, 0.9, 1.0)); // inherited
    }
}
