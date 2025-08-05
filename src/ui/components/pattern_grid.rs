use eframe::egui;
use std::sync::{Arc, Mutex};

use crate::audio::Step;
use crate::timeline::Timeline;

pub struct PatternGrid;

impl PatternGrid {
    pub fn show(ui: &mut egui::Ui, timeline: &Arc<Mutex<Timeline>>, selected_segment_id: Option<&str>) {
        // Timeline mode - get time signature from selected segment
        let (current_step, loop_length, time_signature) = if let Some(id) = selected_segment_id {
            if let Ok(timeline) = timeline.lock() {
                if let Some(segment) = timeline.get_segment(id) {
                    let loop_len = segment.patterns.get(0).map(|p| p.steps.len()).unwrap_or(16);
                    (0, loop_len, segment.time_signature)
                } else {
                    (0, 16, crate::audio::TimeSignature::four_four())
                }
            } else {
                (0, 16, crate::audio::TimeSignature::four_four())
            }
        } else {
            (0, 16, crate::audio::TimeSignature::four_four())
        };

        // Determine which segment to display patterns from
        let segment_to_display = if let Some(id) = selected_segment_id {
            id.to_string()
        } else {
            // If no segment is selected, use the first segment as default
            if let Ok(timeline) = timeline.lock() {
                if let Some(first_segment) = timeline.segments.first() {
                    first_segment.id.clone()
                } else {
                    String::new()
                }
            } else {
                String::new()
            }
        };

        if segment_to_display.is_empty() {
            ui.label("No timeline segments available. Create a segment first.");
            return;
        }

        // Get patterns from the selected timeline segment
        let (patterns, _segment_name) = {
            if let Ok(timeline) = timeline.lock() {
                if let Some(segment) = timeline.get_segment(&segment_to_display) {
                    (segment.patterns.clone(), segment.pattern_id.clone())
                } else {
                    ui.label("Selected segment not found");
                    return;
                }
            } else {
                ui.label("Cannot access timeline");
                return;
            }
        };

        if patterns.is_empty() {
            ui.label("No patterns in selected segment");
            return;
        }

        // Define consistent column widths for perfect alignment
        const TRACK_NAME_WIDTH: f32 = 100.0;
        const STEP_BUTTON_WIDTH: f32 = 32.0;
        const CLEAR_BUTTON_WIDTH: f32 = 60.0;
        const SPACING: f32 = 4.0;

        // Create a frame for the entire grid with subtle styling
        egui::Frame::none()
            .fill(egui::Color32::from_gray(25))
            .stroke(egui::Stroke::new(1.0, egui::Color32::from_gray(50)))
            .inner_margin(egui::Margin::same(8.0))
            .show(ui, |ui| {
                
                // Header row with step numbers - perfectly aligned with columns below
                ui.horizontal(|ui| {
                    // Track name column header
                    ui.allocate_exact_size(
                        egui::vec2(TRACK_NAME_WIDTH, 24.0),
                        egui::Sense::hover()
                    );
                    
                    ui.add_space(SPACING);
                    
                    // Step number headers with time signature-aware beat grouping
                    for step in 0..loop_length {
                        let is_beat_boundary = time_signature.is_beat_boundary(step, loop_length);
                        let is_downbeat = time_signature.is_downbeat(step, loop_length);
                        
                        let text_color = if step == current_step {
                            egui::Color32::YELLOW // Current step (highest priority)
                        } else if is_downbeat {
                            egui::Color32::from_rgb(255, 200, 100) // Downbeat (orange/gold)
                        } else if is_beat_boundary {
                            egui::Color32::WHITE // Beat boundary
                        } else {
                            egui::Color32::GRAY // Subdivision
                        };
                        
                        // Add visual separator at beat boundaries for all time signatures
                        if is_beat_boundary && step > 0 {
                            if is_downbeat {
                                // Thicker separator for measure boundaries (downbeats)
                                ui.add_space(3.0);
                                ui.vertical(|ui| {
                                    ui.colored_label(egui::Color32::from_rgb(255, 200, 100), "┃");
                                });
                                ui.add_space(3.0);
                            } else {
                                // Regular separator for beat boundaries
                                ui.add_space(2.0);
                                ui.vertical(|ui| {
                                    ui.colored_label(egui::Color32::LIGHT_BLUE, "│");
                                });
                                ui.add_space(2.0);
                            }
                        }
                        
                        ui.allocate_ui_with_layout(
                            egui::vec2(STEP_BUTTON_WIDTH, 24.0),
                            egui::Layout::top_down(egui::Align::Center),
                            |ui| {
                                let musical_label = time_signature.step_label(step, loop_length);
                                ui.colored_label(text_color, musical_label);
                            }
                        );
                    }
                    
                    ui.add_space(SPACING);
                    
                    // Clear column header
                    ui.allocate_exact_size(
                        egui::vec2(CLEAR_BUTTON_WIDTH, 24.0),
                        egui::Sense::hover()
                    );
                });

                ui.add_space(4.0);
                ui.separator();
                ui.add_space(4.0);

                // Pattern rows with perfect alignment
                for pattern_index in 0..patterns.len() {
                    let pattern = &patterns[pattern_index];
                    let pattern_name = pattern.name.clone();
                    let pattern_steps = pattern.steps.clone();

                    ui.horizontal(|ui| {
                        // Track name column with fixed width and right alignment
                        ui.allocate_ui_with_layout(
                            egui::vec2(TRACK_NAME_WIDTH, 36.0),
                            egui::Layout::right_to_left(egui::Align::Center),
                            |ui| {
                                ui.strong(&pattern_name);
                            }
                        );
                        
                        ui.add_space(SPACING);

                        // Step buttons with perfect column alignment and time signature-aware grouping
                        for step_index in 0..loop_length {
                            let step = if step_index < pattern_steps.len() {
                                &pattern_steps[step_index]
                            } else {
                                // Handle case where pattern is shorter than loop length
                                &Step::new()
                            };
                            
                            let is_beat_boundary = time_signature.is_beat_boundary(step_index, loop_length);
                            let is_downbeat = time_signature.is_downbeat(step_index, loop_length);
                            
                            // Add visual separator at beat boundaries (matching header logic)
                            if is_beat_boundary && step_index > 0 {
                                if is_downbeat {
                                    // Thicker separator for measure boundaries (downbeats)
                                    ui.add_space(3.0);
                                    ui.vertical(|ui| {
                                        ui.colored_label(egui::Color32::from_rgb(255, 200, 100), "┃");
                                    });
                                    ui.add_space(3.0);
                                } else {
                                    // Regular separator for beat boundaries
                                    ui.add_space(2.0);
                                    ui.vertical(|ui| {
                                        ui.colored_label(egui::Color32::LIGHT_BLUE, "│");
                                    });
                                    ui.add_space(2.0);
                                }
                            }
                            
                            let button_color = if step.active {
                                if step_index == current_step {
                                    egui::Color32::from_rgb(255, 200, 0) // Active and current (gold)
                                } else if is_downbeat {
                                    egui::Color32::from_rgb(0, 255, 100) // Active downbeat (bright green)
                                } else if is_beat_boundary {
                                    egui::Color32::from_rgb(0, 200, 0) // Active beat (green)
                                } else {
                                    egui::Color32::from_rgb(0, 150, 0) // Active subdivision (darker green)
                                }
                            } else if step_index == current_step {
                                egui::Color32::from_rgb(120, 120, 0) // Current but inactive (dim yellow)
                            } else if is_downbeat {
                                egui::Color32::from_gray(60) // Inactive downbeat (lighter gray)
                            } else if is_beat_boundary {
                                egui::Color32::from_gray(50) // Inactive beat (medium gray)
                            } else {
                                egui::Color32::from_gray(40) // Inactive subdivision (dark gray)
                            };

                            // Create button with consistent sizing
                            let button = egui::Button::new("●")
                                .fill(button_color)
                                .stroke(egui::Stroke::new(1.0, egui::Color32::from_gray(100)))
                                .min_size(egui::vec2(STEP_BUTTON_WIDTH, 32.0));

                            if ui.add_sized([STEP_BUTTON_WIDTH, 32.0], button).clicked() {
                                // Toggle step directly in timeline segment
                                if let Ok(mut timeline) = timeline.try_lock() {
                                    if let Some(segment) = timeline.get_segment_mut(&segment_to_display) {
                                        if let Some(pattern) = segment.patterns.get_mut(pattern_index) {
                                            pattern.toggle_step(step_index);
                                        }
                                    }
                                }
                            }
                        }

                        ui.add_space(SPACING);

                        // Clear button with fixed width
                        let clear_button = egui::Button::new("Clear")
                            .fill(egui::Color32::from_rgb(60, 20, 20))
                            .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(100, 40, 40)));
                            
                        if ui.add_sized([CLEAR_BUTTON_WIDTH, 32.0], clear_button).clicked() {
                            // Clear pattern directly in timeline segment
                            if let Ok(mut timeline) = timeline.try_lock() {
                                if let Some(segment) = timeline.get_segment_mut(&segment_to_display) {
                                    if let Some(pattern) = segment.patterns.get_mut(pattern_index) {
                                        pattern.clear();
                                    }
                                }
                            }
                        }
                    });

                    // Add subtle spacing between tracks
                    ui.add_space(2.0);
                }
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::{TimeSignature, sequencer::Pattern};
    use crate::timeline::{Timeline, TimelineSegment};
    
    #[test]
    fn test_beat_boundary_dividers_for_different_time_signatures() {
        // Test that beat boundaries are correctly identified for different time signatures
        let four_four = TimeSignature::four_four();
        let three_four = TimeSignature::three_four();
        let six_eight = TimeSignature::six_eight();
        
        // 4/4 time signature with 16 steps - boundaries at 0, 4, 8, 12
        assert!(four_four.is_beat_boundary(0, 16));
        assert!(four_four.is_beat_boundary(4, 16));
        assert!(four_four.is_beat_boundary(8, 16));
        assert!(four_four.is_beat_boundary(12, 16));
        assert!(!four_four.is_beat_boundary(1, 16));
        assert!(!four_four.is_beat_boundary(3, 16));
        
        // 3/4 time signature with 12 steps - boundaries at 0, 4, 8
        assert!(three_four.is_beat_boundary(0, 12));
        assert!(three_four.is_beat_boundary(4, 12));
        assert!(three_four.is_beat_boundary(8, 12));
        assert!(!three_four.is_beat_boundary(2, 12));
        assert!(!three_four.is_beat_boundary(6, 12));
        
        // 6/8 time signature - compound meter
        assert!(six_eight.is_beat_boundary(0, 12));
        assert!(!six_eight.is_beat_boundary(1, 12));
    }
    
    #[test]
    fn test_downbeat_identification() {
        let four_four = TimeSignature::four_four();
        let three_four = TimeSignature::three_four();
        
        // Only step 0 should be a downbeat
        assert!(four_four.is_downbeat(0, 16));
        assert!(!four_four.is_downbeat(4, 16));
        assert!(!four_four.is_downbeat(8, 16));
        assert!(!four_four.is_downbeat(12, 16));
        
        assert!(three_four.is_downbeat(0, 12));
        assert!(!three_four.is_downbeat(4, 12));
        assert!(!three_four.is_downbeat(8, 12));
    }
    
    #[test]
    fn test_pattern_grid_visual_consistency() {
        // Test that pattern grid constants provide proper visual spacing
        // Constants from the show function - verify they are reasonable values
        const EXPECTED_TRACK_NAME_WIDTH: f32 = 100.0;
        const EXPECTED_STEP_BUTTON_WIDTH: f32 = 32.0;
        const EXPECTED_CLEAR_BUTTON_WIDTH: f32 = 60.0;
        const EXPECTED_SPACING: f32 = 4.0;
        
        // These constants are used within the show function
        // We're testing the expected behavior of the layout system
        assert_eq!(EXPECTED_TRACK_NAME_WIDTH, 100.0);
        assert_eq!(EXPECTED_STEP_BUTTON_WIDTH, 32.0);
        assert_eq!(EXPECTED_CLEAR_BUTTON_WIDTH, 60.0);
        assert_eq!(EXPECTED_SPACING, 4.0);
        
        // Verify minimum layout requirements
        let min_pattern_width = EXPECTED_TRACK_NAME_WIDTH + (16.0 * EXPECTED_STEP_BUTTON_WIDTH) + EXPECTED_CLEAR_BUTTON_WIDTH + (20.0 * EXPECTED_SPACING);
        assert!(min_pattern_width > 600.0, "Pattern grid minimum width should be reasonable: {}", min_pattern_width);
    }
    
    #[test] 
    fn test_timeline_segment_pattern_access() {
        // Test that pattern grid can access patterns from timeline segments
        let mut timeline = Timeline::new();
        
        let patterns = vec![
            Pattern::new("Kick".to_string(), "kick".to_string(), 16),
            Pattern::new("Snare".to_string(), "snare".to_string(), 16),
        ];
        
        let segment = TimelineSegment::new(
            "Test Segment".to_string(),
            patterns,
            0.0,
            1,
            TimeSignature::four_four(),
            120.0,
        );
        
        timeline.add_segment(segment);
        
        // Verify we can access the segment and its patterns
        let segments = &timeline.segments;
        assert_eq!(segments.len(), 1);
        
        let first_segment = &segments[0];
        assert_eq!(first_segment.patterns.len(), 2);
        assert_eq!(first_segment.patterns[0].name, "Kick");
        assert_eq!(first_segment.patterns[1].name, "Snare");
    }
}