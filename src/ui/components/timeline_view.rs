use eframe::egui;
use std::sync::{Arc, Mutex};
use crate::timeline::{Timeline, TimelineSegment, PlaybackState};
use crate::audio::{TimeSignature, sequencer::Pattern};

pub struct TimelineView {
    timeline: Arc<Mutex<Timeline>>,
    zoom_level: f32,           // Pixels per second
    selected_segment: Option<String>,
    drag_state: Option<DragState>,
    scroll_position: f32,      // Horizontal scroll in seconds
    segment_counter: usize,    // Counter for unique segment names
    rename_text: String,       // Text input for renaming
}

#[derive(Debug, Clone)]
struct DragState {
    segment_id: String,
    drag_type: DragType,
    start_mouse_pos: egui::Pos2,
    original_start_time: f64,
    original_duration: f64,
}

#[derive(Debug, Clone, PartialEq)]
enum DragType {
    Move,
    ResizeLeft,
    ResizeRight,
    Split,
}

impl TimelineView {
    pub fn new(timeline: Arc<Mutex<Timeline>>) -> Self {
        Self {
            timeline,
            zoom_level: 50.0, // 50 pixels per second initially
            selected_segment: None,
            drag_state: None,
            scroll_position: 0.0,
            segment_counter: 1, // Start naming from Segment 1
            rename_text: String::new(),
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui, timeline: &Arc<Mutex<Timeline>>) -> bool {
        let mut changed = false;

        // Timeline controls bar
        ui.horizontal(|ui| {
            // Zoom controls
            if ui.button("−").clicked() {
                self.zoom_level = (self.zoom_level * 0.8).max(10.0);
            }
            ui.label(format!("Zoom: {:.0}px/s", self.zoom_level));
            if ui.button("+").clicked() {
                self.zoom_level = (self.zoom_level * 1.25).min(200.0);
            }

            ui.separator();

            // Timeline controls
            if ui.button("Add Segment").clicked() {
                self.add_segment_at_position(0.0);
                changed = true;
            }

            if let Some(selected_id) = self.selected_segment.clone() {
                if ui.button("Duplicate").clicked() {
                    self.duplicate_selected_segment();
                    changed = true;
                }
                if ui.button("Split").clicked() {
                    self.split_selected_segment();
                    changed = true;
                }
                if ui.button("Delete").clicked() {
                    self.delete_selected_segment();
                    changed = true;
                }

                ui.separator();

                // Loop count adjustment
                let current_loop_count = {
                    if let Ok(timeline) = self.timeline.lock() {
                        timeline.get_segment(&selected_id).map(|s| s.loop_count)
                    } else {
                        None
                    }
                };

                if let Some(loop_count) = current_loop_count {
                    ui.label(format!("Loops: {}", loop_count));
                    if ui.button("−").clicked() && loop_count > 1 {
                        self.adjust_segment_loop_count(&selected_id, loop_count - 1);
                        changed = true;
                    }
                    if ui.button("+").clicked() {
                        self.adjust_segment_loop_count(&selected_id, loop_count + 1);
                        changed = true;
                    }

                    ui.separator();

                    // Segment renaming
                    let current_name = {
                        if let Ok(timeline) = self.timeline.lock() {
                            timeline.get_segment(&selected_id).map(|s| s.pattern_id.clone()).unwrap_or_default()
                        } else {
                            String::new()
                        }
                    };

                    if self.rename_text.is_empty() {
                        self.rename_text = current_name.clone();
                    }

                    ui.horizontal(|ui| {
                        ui.label("Name:");
                        if ui.text_edit_singleline(&mut self.rename_text).changed() {
                            // Apply rename immediately
                            let new_name = self.rename_text.trim().to_string();
                            if !new_name.is_empty() {
                                self.rename_selected_segment(&new_name);
                                changed = true;
                            }
                        }
                    });
                }
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Export button
                if ui.button("📁 Export").clicked() {
                    self.export_timeline();
                    changed = true;
                }
            });
        });

        ui.add_space(4.0);

        // Timeline visualization area - full width
        let available_rect = ui.available_rect_before_wrap();
        let timeline_height = 120.0;

        // Create timeline rect that uses full available width
        let timeline_rect = egui::Rect::from_min_size(
            available_rect.min,
            egui::Vec2::new(available_rect.width(), timeline_height)
        );

        // Draw timeline directly without scroll area for now, using full width
        self.draw_timeline(ui, timeline_rect, timeline);

        // Reserve space for the timeline
        ui.allocate_space(egui::Vec2::new(available_rect.width(), timeline_height));

        changed
    }

    fn draw_timeline(&mut self, ui: &mut egui::Ui, rect: egui::Rect, _timeline: &Arc<Mutex<Timeline>>) {
        let painter = ui.painter();

        // Get timeline data
        let (segments, current_position, playback_state, total_duration) = {
            if let Ok(timeline) = self.timeline.lock() {
                (
                    timeline.segments.clone(),
                    timeline.current_position,
                    timeline.playback_state,
                    timeline.total_duration().max(10.0), // Minimum 10 seconds visible
                )
            } else {
                return;
            }
        };

        // Calculate timeline dimensions
        let timeline_width = total_duration as f32 * self.zoom_level;
        let content_rect = egui::Rect::from_min_size(
            rect.min,
            egui::Vec2::new(timeline_width, rect.height())
        );

        // Background
        painter.rect_filled(content_rect, 4.0, egui::Color32::from_gray(25));
        painter.rect_stroke(content_rect, 4.0, egui::Stroke::new(1.0, egui::Color32::from_gray(60)));

        // Time ruler
        self.draw_time_ruler(&painter, content_rect);

        // Segments
        for segment in &segments {
            self.draw_segment(&painter, content_rect, segment, ui);
        }

        // Playback position indicator
        if playback_state == PlaybackState::Playing || playback_state == PlaybackState::Paused {
            let pos_x = content_rect.min.x + (current_position as f32 * self.zoom_level);
            if pos_x >= content_rect.min.x && pos_x <= content_rect.max.x {
                painter.line_segment(
                    [egui::Pos2::new(pos_x, content_rect.min.y), egui::Pos2::new(pos_x, content_rect.max.y)],
                    egui::Stroke::new(2.0, egui::Color32::from_rgb(255, 100, 100))
                );
            }
        }

        // Handle mouse interactions
        let response = ui.allocate_rect(content_rect, egui::Sense::click_and_drag());
        self.handle_mouse_interaction(&response, content_rect);
    }

    fn draw_time_ruler(&self, painter: &egui::Painter, rect: egui::Rect) {
        let ruler_height = 20.0;
        let ruler_rect = egui::Rect::from_min_size(
            rect.min,
            egui::Vec2::new(rect.width(), ruler_height)
        );

        // Ruler background
        painter.rect_filled(ruler_rect, 0.0, egui::Color32::from_gray(35));

        // Time marks
        let seconds_per_mark = if self.zoom_level > 100.0 { 0.5 } else if self.zoom_level > 50.0 { 1.0 } else { 2.0 };
        let _pixels_per_mark = seconds_per_mark * self.zoom_level;

        let start_second = 0.0;
        let end_second = rect.width() / self.zoom_level;

        let mut current_second = (start_second / seconds_per_mark).floor() * seconds_per_mark;
        while current_second <= end_second {
            let x = rect.min.x + (current_second * self.zoom_level);

            // Major marks every few seconds
            let is_major = (current_second % 4.0).abs() < 0.01;
            let mark_height = if is_major { ruler_height * 0.8 } else { ruler_height * 0.5 };
            let color = if is_major { egui::Color32::WHITE } else { egui::Color32::GRAY };

            painter.line_segment(
                [egui::Pos2::new(x, ruler_rect.max.y - mark_height), egui::Pos2::new(x, ruler_rect.max.y)],
                egui::Stroke::new(1.0, color)
            );

            // Time labels on major marks
            if is_major {
                painter.text(
                    egui::Pos2::new(x + 2.0, ruler_rect.min.y + 2.0),
                    egui::Align2::LEFT_TOP,
                    format!("{:.1}s", current_second),
                    egui::FontId::proportional(10.0),
                    egui::Color32::WHITE,
                );
            }

            current_second += seconds_per_mark;
        }
    }

    fn draw_segment(&self, painter: &egui::Painter, rect: egui::Rect, segment: &TimelineSegment, _ui: &egui::Ui) {
        let x_start = rect.min.x + (segment.start_time as f32 * self.zoom_level);
        let x_end = rect.min.x + (segment.end_time() as f32 * self.zoom_level);
        let y_start = rect.min.y + 25.0; // Below ruler
        let segment_height = rect.height() - 30.0;

        let segment_rect = egui::Rect::from_min_max(
            egui::Pos2::new(x_start, y_start),
            egui::Pos2::new(x_end, y_start + segment_height)
        );

        // Only draw if segment is visible
        if segment_rect.max.x < rect.min.x || segment_rect.min.x > rect.max.x {
            return;
        }

        // Segment colors based on selection
        let is_selected = self.selected_segment.as_ref() == Some(&segment.id);
        let (fill_color, stroke_color, stroke_width) = if is_selected {
            (egui::Color32::from_rgb(100, 140, 220), egui::Color32::from_rgb(140, 180, 255), 3.0)
        } else {
            (egui::Color32::from_rgb(60, 80, 120), egui::Color32::from_rgb(100, 120, 160), 2.0)
        };

        // Draw segment rectangle
        painter.rect_filled(segment_rect, 4.0, fill_color);
        painter.rect_stroke(segment_rect, 4.0, egui::Stroke::new(stroke_width, stroke_color));

        // Segment label with pattern step info (count active steps across all patterns)
        let active_steps: usize = segment.patterns.iter()
            .map(|p| p.steps.iter().filter(|s| s.active).count())
            .sum();
        let label_text = format!("{} ({}x) [{}]", segment.pattern_id, segment.loop_count, active_steps);
        painter.text(
            segment_rect.center(),
            egui::Align2::CENTER_CENTER,
            label_text,
            egui::FontId::proportional(11.0),
            egui::Color32::WHITE,
        );

        // Time signature and BPM info (if space allows)
        if segment_rect.width() > 80.0 {
            let info_text = format!("{} @ {:.0} BPM", segment.time_signature.display_string(), segment.bpm);
            painter.text(
                egui::Pos2::new(segment_rect.center().x, segment_rect.center().y + 15.0),
                egui::Align2::CENTER_CENTER,
                info_text,
                egui::FontId::proportional(9.0),
                egui::Color32::from_gray(200),
            );
        }

        // Resize handles for selected segments
        if is_selected && segment_rect.width() > 20.0 {
            let handle_size = 6.0;
            let handle_color = egui::Color32::from_rgb(255, 255, 100);

            // Left handle
            painter.circle_filled(
                egui::Pos2::new(segment_rect.min.x, segment_rect.center().y),
                handle_size,
                handle_color
            );

            // Right handle
            painter.circle_filled(
                egui::Pos2::new(segment_rect.max.x, segment_rect.center().y),
                handle_size,
                handle_color
            );
        }
    }

    fn handle_mouse_interaction(&mut self, response: &egui::Response, rect: egui::Rect) {
        if response.clicked() {
            if let Some(pos) = response.interact_pointer_pos() {
                // Convert mouse position to timeline time
                let timeline_time = ((pos.x - rect.min.x) / self.zoom_level) as f64;

                // Find segment at this position
                let found_segment = {
                    if let Ok(timeline) = self.timeline.lock() {
                        timeline.segments.iter()
                            .find(|s| s.contains_time(timeline_time))
                            .map(|s| s.id.clone())
                    } else {
                        None
                    }
                };

                if let Some(segment_id) = found_segment {
                    self.selected_segment = Some(segment_id.clone());

                    #[cfg(debug_assertions)]
                    eprintln!("DEBUG: Selected timeline segment: {}", segment_id);
                } else {
                    self.selected_segment = None;

                    #[cfg(debug_assertions)]
                    eprintln!("DEBUG: No segment found at time {:.2}s, creating new segment", timeline_time);

                    // Add new segment at clicked position
                    self.add_segment_at_position(timeline_time);
                }
            }
        }

        // Handle drag-and-drop for segment reordering
        if response.dragged() {
            if let Some(selected_id) = &self.selected_segment {
                if let Some(pos) = response.interact_pointer_pos() {
                    let new_time = ((pos.x - rect.min.x) / self.zoom_level).max(0.0) as f64;

                    // Move the selected segment to the new position
                    if let Ok(mut timeline) = self.timeline.lock() {
                        timeline.move_segment(selected_id, new_time);
                    }
                }
            }
        }

        // Handle seeking on playback position (only when no segment is selected)
        if response.clicked() && self.selected_segment.is_none() {
            if let Some(pos) = response.interact_pointer_pos() {
                let timeline_time = ((pos.x - rect.min.x) / self.zoom_level) as f64;
                if let Ok(mut timeline) = self.timeline.lock() {
                    timeline.seek(timeline_time);
                }
            }
        }
    }

    fn add_segment_at_position(&mut self, position: f64) {
        // Create a unique segment name
        let segment_name = format!("Segment {}", self.segment_counter);
        self.segment_counter += 1;

        // Create a full set of empty patterns for all tracks
        let pattern_names = vec!["Kick", "Snare", "Hi-Hat", "Crash", "Open Hi-Hat", "Clap", "Rim Shot", "Tom"];
        let pattern_samples = vec!["kick", "snare", "hihat", "crash", "open_hihat", "clap", "rimshot", "tom"];

        let patterns: Vec<Pattern> = pattern_names.iter().zip(pattern_samples.iter()).map(|(name, sample)| {
            // Create completely empty patterns - no default steps
            Pattern::new(name.to_string(), sample.to_string(), 16)
        }).collect();

        let segment = TimelineSegment::new(
            segment_name,
            patterns,
            position,
            1, // Default to 1 loop
            TimeSignature::four_four(),
            120.0, // Default BPM
        );

        if let Ok(mut timeline) = self.timeline.lock() {
            let id = timeline.add_segment(segment);
            self.selected_segment = Some(id);
        }
    }

    fn duplicate_selected_segment(&mut self) {
        if let Some(selected_id) = &self.selected_segment {
            if let Ok(mut timeline) = self.timeline.lock() {
                // Find a good position for the duplicate (after the original)
                if let Some(original) = timeline.get_segment(selected_id) {
                    let new_start_time = original.end_time() + 0.1; // Small gap

                    // Create a new segment with unique name and copied pattern data
                    let segment_name = format!("Segment {}", self.segment_counter);
                    self.segment_counter += 1;

                    // Clone all patterns and update their names
                    let new_patterns = original.patterns.iter().map(|p| {
                        let mut new_pattern = p.clone();
                        // Update the pattern name to reflect the new segment
                        new_pattern.name = new_pattern.name.replace(&original.pattern_id, &segment_name);
                        new_pattern
                    }).collect();

                    let new_segment = TimelineSegment::new(
                        segment_name,
                        new_patterns,
                        new_start_time,
                        original.loop_count,
                        original.time_signature,
                        original.bpm,
                    );

                    let new_id = timeline.add_segment(new_segment);
                    self.selected_segment = Some(new_id);
                }
            }
        }
    }

    fn split_selected_segment(&mut self) {
        if let Some(selected_id) = &self.selected_segment {
            if let Ok(mut timeline) = self.timeline.lock() {
                if let Some(segment) = timeline.get_segment(selected_id) {
                    let split_time = segment.start_time + (segment.duration / 2.0);

                    // For split segments, we'll use the timeline's built-in split function
                    // but we need to rename the segments afterwards to ensure uniqueness
                    if let Some(new_id) = timeline.split_segment(selected_id, split_time) {
                        // Update the second segment to have a unique name
                        if let Some(new_segment) = timeline.get_segment_mut(&new_id) {
                            let segment_name = format!("Segment {}", self.segment_counter);
                            self.segment_counter += 1;
                            new_segment.pattern_id = segment_name.clone();

                            // Update all pattern names in the new segment
                            for pattern in &mut new_segment.patterns {
                                pattern.name = pattern.name.replace(&new_segment.pattern_id, &segment_name);
                            }
                        }
                        self.selected_segment = Some(new_id);
                    }
                }
            }
        }
    }

    fn delete_selected_segment(&mut self) {
        if let Some(selected_id) = &self.selected_segment {
            if let Ok(mut timeline) = self.timeline.lock() {
                timeline.remove_segment(selected_id);
                self.selected_segment = None;
            }
        }
    }

    fn adjust_segment_loop_count(&mut self, segment_id: &str, new_loop_count: usize) {
        if let Ok(mut timeline) = self.timeline.lock() {
            if let Some(segment) = timeline.get_segment_mut(segment_id) {
                segment.set_loop_count(new_loop_count);
            }
        }
    }

    fn edit_selected_segment_pattern(&mut self) {
        if let Some(selected_id) = &self.selected_segment {
            if let Ok(mut timeline) = self.timeline.lock() {
                if let Some(segment) = timeline.get_segment_mut(selected_id) {
                    // As a demonstration, toggle the first step of the first pattern
                    // In a full implementation, this would open a pattern editor dialog
                    if !segment.patterns.is_empty() && !segment.patterns[0].steps.is_empty() {
                        segment.patterns[0].toggle_step(0);
                    }
                }
            }
        }
    }

    // Method no longer needed since we don't have a separate sequencer to update

    pub fn get_selected_segment_id(&self) -> Option<String> {
        self.selected_segment.clone()
    }

    fn rename_selected_segment(&mut self, new_name: &str) {
        if let Some(selected_id) = &self.selected_segment {
            if let Ok(mut timeline) = self.timeline.lock() {
                if let Some(segment) = timeline.get_segment_mut(selected_id) {
                    let old_name = segment.pattern_id.clone();
                    segment.pattern_id = new_name.to_string();

                    // Update all pattern names in the segment
                    for pattern in &mut segment.patterns {
                        pattern.name = pattern.name.replace(&old_name, new_name);
                    }
                }
            }
        }
    }

    fn export_timeline(&mut self) {
        if let Ok(timeline) = self.timeline.lock() {
            let total_duration = timeline.total_duration();
            let segment_count = timeline.segments.len();

            if segment_count == 0 {
                println!("No segments to export");
                return;
            }

            println!("🎵 Exporting Timeline Composition:");
            println!("   Duration: {:.2}s", total_duration);
            println!("   Segments: {}", segment_count);
            println!("   Format: WAV (simulated)");

            for (i, segment) in timeline.segments.iter().enumerate() {
                let active_steps: usize = segment.patterns.iter()
                    .map(|p| p.steps.iter().filter(|s| s.active).count())
                    .sum();
                println!("   Segment {}: {} ({:.1}s-{:.1}s) - {} loops, {} active steps",
                    i + 1,
                    segment.pattern_id,
                    segment.start_time,
                    segment.end_time(),
                    segment.loop_count,
                    active_steps
                );
            }

            // In a full implementation, this would:
            // 1. Create a temporary audio sequencer
            // 2. Render each segment with its pattern data
            // 3. Concatenate segments into a single audio buffer
            // 4. Export to WAV/MP3 file format
            // 5. Handle different time signatures and BPMs
            println!("✅ Timeline export completed (simulated)");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    #[test]
    fn test_timeline_view_segment_creation() {
        // Create a timeline view
        let timeline = Arc::new(Mutex::new(Timeline::new()));
        let mut timeline_view = TimelineView::new(timeline.clone());

        // Test creating new segments using the add_segment_at_position method
        timeline_view.add_segment_at_position(0.0);
        timeline_view.add_segment_at_position(4.0);

        // Verify segments were created
        let tl = timeline.lock().unwrap();
        assert_eq!(tl.segments.len(), 2);
        
        // Check first segment
        assert_eq!(tl.segments[0].start_time, 0.0);
        assert!(tl.segments[0].patterns.len() > 0);
        
        // Check second segment  
        assert_eq!(tl.segments[1].start_time, 4.0);
        assert!(tl.segments[1].patterns.len() > 0);
        
        println!("✅ Timeline view segment creation test passed");
    }

    #[test]
    fn test_timeline_view_segment_selection() {
        // Create a timeline view with segments
        let timeline = Arc::new(Mutex::new(Timeline::new()));
        let mut timeline_view = TimelineView::new(timeline.clone());
        
        timeline_view.add_segment_at_position(0.0);
        timeline_view.add_segment_at_position(4.0);
        
        // Get segment ID
        let segment_id = {
            let tl = timeline.lock().unwrap();
            tl.segments[0].id.clone()
        };
        
        // Test segment selection
        timeline_view.selected_segment = Some(segment_id.clone());
        assert_eq!(timeline_view.selected_segment, Some(segment_id));
        
        // Test deselection
        timeline_view.selected_segment = None;
        assert_eq!(timeline_view.selected_segment, None);
        
        println!("✅ Timeline view segment selection test passed");
    }
}
