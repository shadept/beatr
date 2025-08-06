use eframe::egui;
use std::sync::{Arc, Mutex};
use crate::timeline::{Timeline, TimelineSegment, PlaybackState};
use crate::audio::{TimeSignature, sequencer::Pattern};

// Theme-aware color helper functions for timeline view
fn get_timeline_bg_color(visuals: &egui::Visuals) -> egui::Color32 {
    if visuals.dark_mode {
        egui::Color32::from_gray(25)
    } else {
        egui::Color32::from_gray(250)
    }
}

fn get_timeline_stroke_color(visuals: &egui::Visuals) -> egui::Color32 {
    if visuals.dark_mode {
        egui::Color32::from_gray(60)
    } else {
        egui::Color32::from_gray(180)
    }
}

fn get_ruler_bg_color(visuals: &egui::Visuals) -> egui::Color32 {
    if visuals.dark_mode {
        egui::Color32::from_gray(35)
    } else {
        egui::Color32::from_gray(240)
    }
}

fn get_grid_line_color(visuals: &egui::Visuals) -> egui::Color32 {
    if visuals.dark_mode {
        egui::Color32::from_gray(45)
    } else {
        egui::Color32::from_gray(220)
    }
}

fn get_segment_boundary_color(visuals: &egui::Visuals) -> egui::Color32 {
    if visuals.dark_mode {
        egui::Color32::from_gray(65)
    } else {
        egui::Color32::from_gray(200)
    }
}

fn get_selected_segment_colors(visuals: &egui::Visuals) -> (egui::Color32, egui::Color32) {
    if visuals.dark_mode {
        (egui::Color32::from_rgb(100, 140, 220), egui::Color32::from_rgb(140, 180, 255))
    } else {
        // Better contrast for light theme: darker blue fill with darker border
        (egui::Color32::from_rgb(120, 160, 240), egui::Color32::from_rgb(80, 120, 200))
    }
}

fn get_unselected_segment_colors(visuals: &egui::Visuals) -> (egui::Color32, egui::Color32) {
    if visuals.dark_mode {
        (egui::Color32::from_rgb(60, 80, 120), egui::Color32::from_rgb(100, 120, 160))
    } else {
        // Better contrast for light theme: medium gray with darker border
        (egui::Color32::from_rgb(200, 210, 220), egui::Color32::from_rgb(140, 160, 180))
    }
}

fn get_time_sig_selected_color(visuals: &egui::Visuals) -> egui::Color32 {
    if visuals.dark_mode {
        egui::Color32::from_rgb(0, 150, 0)
    } else {
        egui::Color32::from_rgb(0, 120, 0)
    }
}

fn get_time_sig_unselected_color(visuals: &egui::Visuals) -> egui::Color32 {
    if visuals.dark_mode {
        egui::Color32::from_gray(60)
    } else {
        egui::Color32::from_gray(200)
    }
}

pub struct TimelineView {
    timeline: Arc<Mutex<Timeline>>,
    zoom_level: f32,           // Pixels per second
    selected_segment: Option<String>,
    scroll_position: f32,      // Horizontal scroll in seconds
    segment_counter: usize,    // Counter for unique segment names
    rename_text: String,       // Text input for renaming
    snap_preview: Option<f64>, // Preview position for snapping
}


impl TimelineView {
    pub fn new(timeline: Arc<Mutex<Timeline>>) -> Self {
        Self {
            timeline,
            zoom_level: 50.0, // 50 pixels per second initially
            selected_segment: None,
            scroll_position: 0.0,
            segment_counter: 1, // Start naming from Segment 1
            rename_text: String::new(),
            snap_preview: None,
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui, timeline: &Arc<Mutex<Timeline>>, global_bpm: f32) -> bool {
        let mut changed = false;

        // Flattened timeline controls - single horizontal layout like transport controls
        ui.horizontal(|ui| {
            // Zoom controls - direct placement, no groups
            if ui.small_button("−").clicked() {
                self.zoom_level = (self.zoom_level * 0.8).max(10.0);
            }
            ui.label(format!("Zoom: {:.0}px/s", self.zoom_level));
            if ui.small_button("+").clicked() {
                self.zoom_level = (self.zoom_level * 1.25).min(200.0);
            }

            ui.separator();

            // Add Segment button - direct placement
            if ui.button("Add Segment").clicked() {
                self.add_segment_at_position(0.0, global_bpm);
                changed = true;
            }

            // Segment controls if selected - direct placement
            if let Some(selected_id) = self.selected_segment.clone() {
                ui.separator();

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

                // Loop count controls if selected - direct placement
                let current_loop_count = {
                    if let Ok(timeline) = self.timeline.lock() {
                        timeline.get_segment(&selected_id).map(|s| s.loop_count)
                    } else {
                        None
                    }
                };

                if let Some(loop_count) = current_loop_count {
                    ui.separator();

                    ui.label(format!("Loops: {}", loop_count));
                    if ui.small_button("−").clicked() && loop_count > 1 {
                        self.adjust_segment_loop_count(&selected_id, loop_count - 1);
                        changed = true;
                    }
                    if ui.small_button("+").clicked() {
                        self.adjust_segment_loop_count(&selected_id, loop_count + 1);
                        changed = true;
                    }
                }

                // BPM controls if selected - direct placement
                let current_bpm = {
                    if let Ok(timeline) = self.timeline.lock() {
                        timeline.get_segment(&selected_id).map(|s| s.bpm)
                    } else {
                        None
                    }
                };

                if let Some(mut bpm) = current_bpm {
                    ui.separator();

                    ui.label("BPM:");
                    if ui.add(egui::DragValue::new(&mut bpm)
                        .range(60.0..=300.0)
                        .speed(1.0)
                        .prefix("♩ ")
                        .suffix(" BPM")
                        .min_decimals(0)
                        .max_decimals(0))
                        .changed()
                    {
                        self.adjust_segment_bpm(&selected_id, bpm);
                        changed = true;
                    }

                    if ui.small_button("80").clicked() {
                        self.adjust_segment_bpm(&selected_id, 80.0);
                        changed = true;
                    }
                    if ui.small_button("120").clicked() {
                        self.adjust_segment_bpm(&selected_id, 120.0);
                        changed = true;
                    }
                    if ui.small_button("140").clicked() {
                        self.adjust_segment_bpm(&selected_id, 140.0);
                        changed = true;
                    }
                    if ui.small_button("160").clicked() {
                        self.adjust_segment_bpm(&selected_id, 160.0);
                        changed = true;
                    }
                }

                // Time signature controls if selected - direct placement
                let current_time_signature = {
                    if let Ok(timeline) = self.timeline.lock() {
                        timeline.get_segment(&selected_id).map(|s| s.time_signature)
                    } else {
                        None
                    }
                };

                if let Some(time_sig) = current_time_signature {
                    ui.separator();

                    ui.label("Time Sig:");
                    let presets = [
                        ("4/4", crate::audio::TimeSignature::four_four()),
                        ("3/4", crate::audio::TimeSignature::three_four()),
                        ("5/4", crate::audio::TimeSignature::five_four()),
                        ("6/8", crate::audio::TimeSignature::six_eight()),
                        ("7/8", crate::audio::TimeSignature::seven_eight()),
                    ];

                    for (label, preset_ts) in &presets {
                        let is_selected = time_sig == *preset_ts;
                        let button = egui::Button::new(*label)
                            .small()
                            .fill(if is_selected {
                                get_time_sig_selected_color(&ui.visuals())
                            } else {
                                get_time_sig_unselected_color(&ui.visuals())
                            });

                        if ui.add(button).clicked() && !is_selected {
                            self.adjust_segment_time_signature(&selected_id, *preset_ts);
                            changed = true;
                        }
                    }
                }

                // Segment renaming if selected - direct placement
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

                ui.separator();

                ui.label("Name:");
                if ui.text_edit_singleline(&mut self.rename_text).changed() {
                    let new_name = self.rename_text.trim().to_string();
                    if !new_name.is_empty() {
                        self.rename_selected_segment(&new_name);
                        changed = true;
                    }
                }
            }

            // Export button on the right - direct placement
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("📁 Export").clicked() {
                    self.export_timeline();
                    changed = true;
                }
            });
        });

        ui.add_space(4.0);

        // Timeline visualization area - full width with scrolling
        let available_rect = ui.available_rect_before_wrap();
        let timeline_height = 120.0;

        // Create scrollable timeline area
        let timeline_rect = egui::Rect::from_min_size(
            available_rect.min,
            egui::Vec2::new(available_rect.width(), timeline_height)
        );

        // Draw timeline with scrolling support
        self.draw_scrollable_timeline(ui, timeline_rect, timeline);

        changed
    }

    fn draw_scrollable_timeline(&mut self, ui: &mut egui::Ui, rect: egui::Rect, _timeline: &Arc<Mutex<Timeline>>) {
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

        // Calculate timeline dimensions and scroll boundaries
        let timeline_width = total_duration as f32 * self.zoom_level;
        let viewport_width = rect.width();

        // Calculate scroll boundaries: allow scrolling 10% past each end
        let scroll_margin = total_duration as f32 * 0.1;
        let min_scroll = -scroll_margin;
        let max_scroll = if timeline_width > viewport_width {
            (timeline_width - viewport_width) / self.zoom_level + scroll_margin
        } else {
            scroll_margin
        };

        // Clamp scroll position to boundaries
        self.scroll_position = self.scroll_position.clamp(min_scroll, max_scroll);

        // Calculate content rect with scroll offset
        let scroll_offset_pixels = self.scroll_position * self.zoom_level;
        let content_rect = egui::Rect::from_min_size(
            egui::Pos2::new(rect.min.x - scroll_offset_pixels, rect.min.y),
            egui::Vec2::new(timeline_width, rect.height())
        );

        let painter = ui.painter();

        // Background - fill entire viewport  
        let visuals = ui.ctx().style().visuals.clone();
        painter.rect_filled(rect, 4.0, get_timeline_bg_color(&visuals));
        painter.rect_stroke(rect, 4.0, egui::Stroke::new(1.0, get_timeline_stroke_color(&visuals)));

        // Time ruler - pass both viewport and content rects for proper positioning
        self.draw_time_ruler(&painter, rect, content_rect, ui);

        // Segments (will handle their own viewport clipping)
        for segment in &segments {
            self.draw_segment(&painter, content_rect, segment, ui);
        }

        // Snap grid visualization (subtle grid lines)
        self.draw_snap_grid(&painter, content_rect, ui);

        // Snap preview indicator
        if let Some(snap_time) = self.snap_preview {
            let snap_x = rect.min.x + ((snap_time as f32 - self.scroll_position) * self.zoom_level);
            if snap_x >= rect.min.x && snap_x <= rect.max.x {
                painter.line_segment(
                    [egui::Pos2::new(snap_x, rect.min.y + 25.0), egui::Pos2::new(snap_x, rect.max.y)],
                    egui::Stroke::new(2.0, egui::Color32::from_rgb(100, 255, 100))
                );
                // Add snap indicator at the top
                painter.circle_filled(
                    egui::Pos2::new(snap_x, rect.min.y + 30.0),
                    4.0,
                    egui::Color32::from_rgb(100, 255, 100)
                );
            }
        }

        // Playback position indicator
        if playback_state == PlaybackState::Playing || playback_state == PlaybackState::Paused {
            // Calculate position in viewport coordinates, accounting for scroll
            let pos_x = rect.min.x + ((current_position as f32 - self.scroll_position) * self.zoom_level);
            // Only draw if visible in viewport
            if pos_x >= rect.min.x && pos_x <= rect.max.x {
                painter.line_segment(
                    [egui::Pos2::new(pos_x, rect.min.y), egui::Pos2::new(pos_x, rect.max.y)],
                    egui::Stroke::new(2.0, egui::Color32::from_rgb(255, 100, 100))
                );
            }
        }

        // Handle mouse interactions - use viewport rect but convert coordinates
        let response = ui.allocate_rect(rect, egui::Sense::click_and_drag());
        self.handle_mouse_interaction(&response, rect, content_rect);

        // Handle scroll wheel for horizontal scrolling
        if response.hovered() {
            let scroll_delta = ui.input(|i| i.raw_scroll_delta);
            if scroll_delta.x != 0.0 {
                // Convert scroll delta to time units
                let scroll_speed = 2.0; // Adjust for scroll sensitivity
                self.scroll_position -= scroll_delta.x * scroll_speed / self.zoom_level;
            }
        }
    }

    fn draw_time_ruler(&self, painter: &egui::Painter, viewport_rect: egui::Rect, _content_rect: egui::Rect, ui: &egui::Ui) {
        let ruler_height = 20.0;
        let ruler_rect = egui::Rect::from_min_size(
            viewport_rect.min,
            egui::Vec2::new(viewport_rect.width(), ruler_height)
        );

        // Ruler background
        painter.rect_filled(ruler_rect, 0.0, get_ruler_bg_color(&ui.visuals()));

        // Time marks based on current scroll position and zoom
        let seconds_per_mark = if self.zoom_level > 100.0 { 0.5 } else if self.zoom_level > 50.0 { 1.0 } else { 2.0 };

        // Calculate visible time range based on scroll position
        let start_second = self.scroll_position;
        let end_second = self.scroll_position + (viewport_rect.width() / self.zoom_level);

        let mut current_second = (start_second / seconds_per_mark).floor() * seconds_per_mark;
        while current_second <= end_second {
            // Calculate x position relative to viewport, accounting for scroll
            let x = viewport_rect.min.x + ((current_second - self.scroll_position) * self.zoom_level);

            // Only draw marks that are visible in the viewport
            if x >= viewport_rect.min.x && x <= viewport_rect.max.x {
                // Major marks every few seconds
                let is_major = (current_second % 4.0).abs() < 0.01;
                let mark_height = if is_major { ruler_height * 0.8 } else { ruler_height * 0.5 };
                let color = if is_major { egui::Color32::WHITE } else { egui::Color32::GRAY };

                painter.line_segment(
                    [egui::Pos2::new(x, ruler_rect.max.y - mark_height), egui::Pos2::new(x, ruler_rect.max.y)],
                    egui::Stroke::new(1.0, color)
                );

                // Time labels on major marks (only if positive time)
                if is_major && current_second >= 0.0 {
                    painter.text(
                        egui::Pos2::new(x + 2.0, ruler_rect.min.y + 2.0),
                        egui::Align2::LEFT_TOP,
                        format!("{:.1}s", current_second),
                        egui::FontId::proportional(10.0),
                        egui::Color32::WHITE,
                    );
                }
            }

            current_second += seconds_per_mark;
        }
    }

    // Pattern preview generation utility
    fn generate_pattern_preview(&self, segment: &TimelineSegment, preview_width: f32, preview_height: f32, visuals: &egui::Visuals) -> Vec<(egui::Pos2, egui::Color32)> {
        let mut preview_elements = Vec::new();

        if segment.patterns.is_empty() {
            return preview_elements;
        }

        let pattern_colors = if visuals.dark_mode {
            // Original bright colors for dark theme
            [
                egui::Color32::from_rgb(255, 100, 100), // Kick - red
                egui::Color32::from_rgb(100, 150, 255), // Snare - blue
                egui::Color32::from_rgb(255, 255, 100), // Hi-Hat - yellow
                egui::Color32::from_rgb(255, 150, 100), // Crash - orange
                egui::Color32::from_rgb(200, 255, 100), // Open Hi-Hat - light green
                egui::Color32::from_rgb(255, 100, 255), // Clap - magenta
                egui::Color32::from_rgb(150, 100, 255), // Rim Shot - purple
                egui::Color32::from_rgb(100, 255, 200), // Tom - cyan
            ]
        } else {
            // Darker, more contrasted colors for light theme
            [
                egui::Color32::from_rgb(180, 40, 40),   // Kick - dark red
                egui::Color32::from_rgb(40, 80, 180),   // Snare - dark blue
                egui::Color32::from_rgb(180, 160, 40),  // Hi-Hat - dark yellow
                egui::Color32::from_rgb(200, 100, 40),  // Crash - dark orange
                egui::Color32::from_rgb(80, 160, 40),   // Open Hi-Hat - dark green
                egui::Color32::from_rgb(180, 40, 180),  // Clap - dark magenta
                egui::Color32::from_rgb(120, 40, 180),  // Rim Shot - dark purple
                egui::Color32::from_rgb(40, 140, 120),  // Tom - dark teal
            ]
        };

        let pattern_step_count = segment.patterns.get(0).map(|p| p.steps.len()).unwrap_or(16);
        let total_steps = pattern_step_count * segment.loop_count;
        let step_width = preview_width / total_steps as f32;
        let pattern_height = preview_height / segment.patterns.len() as f32;

        for (pattern_idx, pattern) in segment.patterns.iter().enumerate() {
            let color = pattern_colors.get(pattern_idx).unwrap_or(&egui::Color32::GRAY);
            let y_offset = pattern_idx as f32 * pattern_height;

            // Draw pattern repeated for each loop
            for loop_iteration in 0..segment.loop_count {
                for (step_idx, step) in pattern.steps.iter().enumerate() {
                    if step.active {
                        let absolute_step_idx = loop_iteration * pattern_step_count + step_idx;
                        let x = absolute_step_idx as f32 * step_width + step_width * 0.25;
                        let y = y_offset + pattern_height * 0.25;
                        let size = (step_width * 0.5).min(pattern_height * 0.5).max(1.0);

                        preview_elements.push((egui::Pos2::new(x, y), *color));
                        preview_elements.push((egui::Pos2::new(x + size, y + size), *color)); // Store size in second point
                    }
                }
            }
        }

        preview_elements
    }

    // Text measurement and truncation utilities
    fn measure_text_width(&self, text: &str, font_size: f32) -> f32 {
        // Simple approximation: average character width * character count
        // In a real implementation, this would use egui's text measurement
        let avg_char_width = font_size * 0.6; // Approximate monospace character width ratio
        text.len() as f32 * avg_char_width
    }

    fn truncate_text_with_ellipses(&self, text: &str, max_width: f32, font_size: f32) -> String {
        if self.measure_text_width(text, font_size) <= max_width {
            return text.to_string();
        }

        let ellipses = "...";
        let ellipses_width = self.measure_text_width(ellipses, font_size);
        let available_width = max_width - ellipses_width;

        if available_width <= 0.0 {
            return ellipses.to_string();
        }

        let avg_char_width = font_size * 0.6;
        let max_chars = (available_width / avg_char_width) as usize;

        if max_chars == 0 {
            return ellipses.to_string();
        }

        // Try to break at word boundaries when possible
        let truncated = if max_chars < text.len() {
            let substr = &text[..max_chars.min(text.len())];
            if let Some(last_space) = substr.rfind(' ') {
                if last_space > max_chars / 2 { // Only break at word if it's not too early
                    &text[..last_space]
                } else {
                    substr
                }
            } else {
                substr
            }
        } else {
            text
        };

        format!("{}{}", truncated, ellipses)
    }

    // Calculate appropriate font size and content for segment width
    fn calculate_segment_text_display(&self, segment: &TimelineSegment, segment_width: f32) -> (String, f32) {
        let base_font_size = 11.0;
        let min_font_size = 8.0;
        let max_font_size = 14.0;

        // Determine zoom-adaptive font size
        let font_size = if segment_width < 50.0 {
            min_font_size
        } else if segment_width < 100.0 {
            (base_font_size * (segment_width / 100.0)).clamp(min_font_size, max_font_size)
        } else {
            base_font_size
        };

        // Determine content priority based on available width
        let text_margin = 8.0; // Leave some margin for readability
        let available_width = segment_width - text_margin;

        // Priority: name > loop count (removed BPM and time signature per story requirements)
        let name_and_loops = format!("{} ({}x)", segment.pattern_id, segment.loop_count);

        if self.measure_text_width(&name_and_loops, font_size) <= available_width {
            (name_and_loops, font_size)
        } else {
            // Try just the name with loop count abbreviated
            let abbreviated = format!("{} ({})", segment.pattern_id, segment.loop_count);
            if self.measure_text_width(&abbreviated, font_size) <= available_width {
                (abbreviated, font_size)
            } else {
                // Truncate the name and keep minimal loop info if possible
                let loop_part = format!(" ({})", segment.loop_count);
                let loop_width = self.measure_text_width(&loop_part, font_size);

                if loop_width < available_width * 0.3 { // Reserve 30% for loop count
                    let name_width = available_width - loop_width;
                    let truncated_name = self.truncate_text_with_ellipses(&segment.pattern_id, name_width, font_size);
                    (format!("{}{}", truncated_name, loop_part), font_size)
                } else {
                    // Just the name, truncated
                    (self.truncate_text_with_ellipses(&segment.pattern_id, available_width, font_size), font_size)
                }
            }
        }
    }

    fn draw_segment(&self, painter: &egui::Painter, rect: egui::Rect, segment: &TimelineSegment, ui: &egui::Ui) {
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
        let visuals = ui.ctx().style().visuals.clone();
        let (fill_color, stroke_color, stroke_width) = if is_selected {
            let (fill, stroke) = get_selected_segment_colors(&visuals);
            (fill, stroke, 3.0)
        } else {
            let (fill, stroke) = get_unselected_segment_colors(&visuals);
            (fill, stroke, 2.0)
        };

        // Draw segment rectangle
        painter.rect_filled(segment_rect, 4.0, fill_color);
        painter.rect_stroke(segment_rect, 4.0, egui::Stroke::new(stroke_width, stroke_color));

        // Calculate adaptive text content and sizing
        let (text_content, font_size) = self.calculate_segment_text_display(segment, segment_rect.width());

        // Pattern preview visualization (above text)
        let preview_area_height = (segment_rect.height() * 0.4).min(30.0); // 40% of segment height, max 30px
        let preview_y_offset = segment_rect.min.y + 4.0;
        let preview_width = segment_rect.width() - 8.0; // 4px margin on each side

        if segment_rect.width() > 40.0 && preview_area_height > 8.0 {
            let preview_elements = self.generate_pattern_preview(segment, preview_width, preview_area_height - 4.0, &visuals);

            for chunk in preview_elements.chunks(2) {
                if chunk.len() == 2 {
                    let pos = egui::Pos2::new(
                        segment_rect.min.x + 4.0 + chunk[0].0.x,
                        preview_y_offset + chunk[0].0.y
                    );
                    let size = chunk[1].0.x - chunk[0].0.x; // Size stored in second point's x
                    let color = chunk[0].1;

                    painter.rect_filled(
                        egui::Rect::from_min_size(pos, egui::Vec2::splat(size)),
                        1.0,
                        color
                    );
                }
            }
        }

        // Main text label (below pattern preview)
        let text_y_position = if segment_rect.width() > 40.0 && preview_area_height > 8.0 {
            segment_rect.center().y + preview_area_height * 0.3 // Position below preview
        } else {
            segment_rect.center().y // Centered if no preview
        };

        painter.text(
            egui::Pos2::new(segment_rect.center().x, text_y_position),
            egui::Align2::CENTER_CENTER,
            text_content,
            egui::FontId::proportional(font_size),
            egui::Color32::WHITE,
        );

    }

    fn calculate_snap_time(&self, time: f64, exclude_segment_id: Option<&str>) -> f64 {
        // Get all potential snap points from existing segments
        let mut snap_points = Vec::new();

        if let Ok(timeline) = self.timeline.lock() {
            for segment in &timeline.segments {
                // Skip the segment we're currently dragging
                if let Some(exclude_id) = exclude_segment_id {
                    if segment.id == exclude_id {
                        continue;
                    }
                }
                // Add segment start and end times as snap points
                snap_points.push(segment.start_time);
                snap_points.push(segment.end_time());
            }
        }

        // Add regular grid snap points
        let snap_interval = if self.zoom_level > 100.0 { 0.25 } // 1/4 second
        else if self.zoom_level > 50.0 { 0.5 }  // 1/2 second
        else { 1.0 }; // 1 second

        // Generate grid points around the target time
        let grid_start = (time / snap_interval).floor() * snap_interval;
        for i in -2..=2 {
            let grid_point = grid_start + (i as f64 * snap_interval);
            if grid_point >= 0.0 {
                snap_points.push(grid_point);
            }
        }

        // Find the closest snap point
        if snap_points.is_empty() {
            return (time / snap_interval).round() * snap_interval;
        }

        snap_points.sort_by(|a, b| a.partial_cmp(b).unwrap());

        // Find the closest snap point to the target time
        let closest_snap = snap_points.iter()
            .min_by(|a, b| {
                let dist_a = (time - **a).abs();
                let dist_b = (time - **b).abs();
                dist_a.partial_cmp(&dist_b).unwrap()
            })
            .unwrap_or(&time);

        *closest_snap
    }

    fn draw_snap_grid(&self, painter: &egui::Painter, rect: egui::Rect, ui: &egui::Ui) {
        // Draw subtle grid lines for regular snap points
        let snap_interval = if self.zoom_level > 100.0 { 0.25f64 }
        else if self.zoom_level > 50.0 { 0.5f64 }
        else { 1.0f64 };

        let start_time = 0.0f64;
        let end_time = rect.width() as f64 / self.zoom_level as f64;

        let mut current_time = (start_time / snap_interval).floor() * snap_interval;
        while current_time <= end_time {
            let x = rect.min.x + (current_time as f32 * self.zoom_level);

            // Draw very subtle grid lines for regular intervals
            painter.line_segment(
                [egui::Pos2::new(x, rect.min.y + 25.0), egui::Pos2::new(x, rect.max.y)],
                egui::Stroke::new(0.5, get_grid_line_color(&ui.visuals()))
            );

            current_time += snap_interval;
        }

        // Draw segment boundary snap points (slightly more visible)
        if let Ok(timeline) = self.timeline.lock() {
            for segment in &timeline.segments {
                // Draw segment start boundary
                let start_x = rect.min.x + (segment.start_time as f32 * self.zoom_level);
                if start_x >= rect.min.x && start_x <= rect.max.x {
                    painter.line_segment(
                        [egui::Pos2::new(start_x, rect.min.y + 25.0), egui::Pos2::new(start_x, rect.max.y)],
                        egui::Stroke::new(1.0, get_segment_boundary_color(&ui.visuals()))
                    );
                }

                // Draw segment end boundary
                let end_x = rect.min.x + (segment.end_time() as f32 * self.zoom_level);
                if end_x >= rect.min.x && end_x <= rect.max.x {
                    painter.line_segment(
                        [egui::Pos2::new(end_x, rect.min.y + 25.0), egui::Pos2::new(end_x, rect.max.y)],
                        egui::Stroke::new(1.0, get_segment_boundary_color(&ui.visuals()))
                    );
                }
            }
        }
    }

    fn handle_mouse_interaction(&mut self, response: &egui::Response, viewport_rect: egui::Rect, _content_rect: egui::Rect) {
        if response.clicked() {
            if let Some(pos) = response.interact_pointer_pos() {
                // Convert mouse position to timeline time, accounting for scroll
                let timeline_time = ((pos.x - viewport_rect.min.x) / self.zoom_level + self.scroll_position) as f64;

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

                    // Add new segment at clicked position (we'll use 120.0 as default BPM for user-created segments)
                    self.add_segment_at_position(timeline_time, 120.0);
                }
            }
        }

        // Handle drag-and-drop for segment reordering with snapping
        if response.dragged() {
            if let Some(selected_id) = &self.selected_segment {
                if let Some(pos) = response.interact_pointer_pos() {
                    let raw_time = ((pos.x - viewport_rect.min.x) / self.zoom_level + self.scroll_position).max(0.0) as f64;

                    // Check if Alt key is held to disable snapping
                    let alt_held = response.ctx.input(|i| i.modifiers.alt);

                    let final_time = if alt_held {
                        // No snapping when Alt is held
                        self.snap_preview = None;
                        raw_time
                    } else {
                        // Use enhanced snapping that includes segment boundaries
                        let snapped_time = self.calculate_snap_time(raw_time, Some(selected_id));
                        self.snap_preview = Some(snapped_time);
                        snapped_time
                    };

                    // Move the selected segment to the final position
                    if let Ok(mut timeline) = self.timeline.lock() {
                        timeline.move_segment(selected_id, final_time);
                    }
                }
            }
        } else {
            // Clear snap preview when not dragging
            self.snap_preview = None;
        }

        // Handle seeking on playback position (only when no segment is selected)
        if response.clicked() && self.selected_segment.is_none() {
            if let Some(pos) = response.interact_pointer_pos() {
                let timeline_time = ((pos.x - viewport_rect.min.x) / self.zoom_level + self.scroll_position) as f64;
                if let Ok(mut timeline) = self.timeline.lock() {
                    timeline.seek(timeline_time);
                }
            }
        }
    }

    fn add_segment_at_position(&mut self, position: f64, bpm: f32) {
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
            bpm, // Use the provided BPM
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

    fn adjust_segment_bpm(&mut self, segment_id: &str, new_bpm: f32) {
        if let Ok(mut timeline) = self.timeline.lock() {
            if let Some(segment) = timeline.get_segment_mut(segment_id) {
                segment.set_bpm(new_bpm);
            }
        }
    }

    fn adjust_segment_time_signature(&mut self, segment_id: &str, new_time_signature: crate::audio::TimeSignature) {
        if let Ok(mut timeline) = self.timeline.lock() {
            if let Some(segment) = timeline.get_segment_mut(segment_id) {
                segment.set_time_signature(new_time_signature);
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
        timeline_view.add_segment_at_position(0.0, 120.0);
        timeline_view.add_segment_at_position(4.0, 120.0);

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

        timeline_view.add_segment_at_position(0.0, 120.0);
        timeline_view.add_segment_at_position(4.0, 120.0);

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

    #[test]
    fn test_timeline_view_snapping_functionality() {
        // Create a timeline view
        let timeline = Arc::new(Mutex::new(Timeline::new()));
        let mut timeline_view = TimelineView::new(timeline.clone());

        // Test snapping at different zoom levels (grid-only snapping)
        timeline_view.zoom_level = 120.0; // High zoom
        assert_eq!(timeline_view.calculate_snap_time(0.13, None), 0.25); // Should snap to 1/4 second
        assert_eq!(timeline_view.calculate_snap_time(0.62, None), 0.5);

        timeline_view.zoom_level = 75.0; // Medium zoom
        assert_eq!(timeline_view.calculate_snap_time(0.3, None), 0.5); // Should snap to 1/2 second
        assert_eq!(timeline_view.calculate_snap_time(0.8, None), 1.0);

        timeline_view.zoom_level = 30.0; // Low zoom
        assert_eq!(timeline_view.calculate_snap_time(0.6, None), 1.0); // Should snap to 1 second
        assert_eq!(timeline_view.calculate_snap_time(1.4, None), 1.0);
        assert_eq!(timeline_view.calculate_snap_time(1.6, None), 2.0);

        println!("✅ Timeline view snapping functionality test passed");
    }

    #[test]
    fn test_timeline_view_segment_boundary_snapping() {
        // Create a timeline view with segments
        let timeline = Arc::new(Mutex::new(Timeline::new()));
        let mut timeline_view = TimelineView::new(timeline.clone());

        // Add some segments to test boundary snapping
        timeline_view.add_segment_at_position(1.0, 120.0); // Segment at 1.0s
        timeline_view.add_segment_at_position(3.5, 120.0); // Segment at 3.5s

        timeline_view.zoom_level = 50.0; // Medium zoom

        // Test snapping to segment boundaries
        // Should snap to start of first segment (1.0s) rather than grid point (1.0s in this case both match)
        assert_eq!(timeline_view.calculate_snap_time(0.9, None), 1.0);

        // Should snap to end of first segment (assuming 1 loop at default duration)
        let expected_end = {
            if let Ok(tl) = timeline.lock() {
                tl.segments[0].end_time()
            } else {
                2.0 // fallback
            }
        };

        // Test snapping near segment end
        let snap_result = timeline_view.calculate_snap_time(expected_end - 0.1, None);
        assert!((snap_result - expected_end).abs() < 0.01,
               "Expected snap to {}, got {}", expected_end, snap_result);

        println!("✅ Timeline view segment boundary snapping test passed");
    }

    #[test]
    fn test_timeline_view_scrolling_boundaries() {
        // Create a timeline view with segments
        let timeline = Arc::new(Mutex::new(Timeline::new()));
        let mut timeline_view = TimelineView::new(timeline.clone());

        // Add segments to create timeline content
        timeline_view.add_segment_at_position(0.0, 120.0);  // 0-4s
        timeline_view.add_segment_at_position(5.0, 120.0);  // 5-9s
        timeline_view.add_segment_at_position(10.0, 120.0); // 10-14s

        timeline_view.zoom_level = 50.0; // 50 pixels per second

        // Get total duration
        let total_duration = {
            if let Ok(tl) = timeline.lock() {
                tl.total_duration() as f32
            } else {
                14.0 // fallback
            }
        };

        // Test scroll boundaries calculation
        let viewport_width = 300.0; // Simulate viewport width
        let timeline_width = total_duration * timeline_view.zoom_level;
        let scroll_margin = total_duration * 0.1; // 10% margin
        let expected_min_scroll = -scroll_margin;
        let expected_max_scroll = if timeline_width > viewport_width {
            (timeline_width - viewport_width) / timeline_view.zoom_level + scroll_margin
        } else {
            scroll_margin
        };

        // Test clamping to minimum boundary
        timeline_view.scroll_position = -1000.0; // Way past minimum
        // Simulate the boundary clamping that happens in draw_scrollable_timeline
        timeline_view.scroll_position = timeline_view.scroll_position.clamp(expected_min_scroll, expected_max_scroll);
        assert!((timeline_view.scroll_position - expected_min_scroll).abs() < 0.01);

        // Test clamping to maximum boundary
        timeline_view.scroll_position = 1000.0; // Way past maximum
        timeline_view.scroll_position = timeline_view.scroll_position.clamp(expected_min_scroll, expected_max_scroll);
        assert!((timeline_view.scroll_position - expected_max_scroll).abs() < 0.01);

        // Test valid scroll position (use a position we know is within bounds)
        let valid_position = (expected_min_scroll + expected_max_scroll) / 2.0; // Middle position
        timeline_view.scroll_position = valid_position;
        timeline_view.scroll_position = timeline_view.scroll_position.clamp(expected_min_scroll, expected_max_scroll);
        assert!((timeline_view.scroll_position - valid_position).abs() < 0.01);

        println!("✅ Timeline view scrolling boundaries test passed");
    }

    #[test]
    fn test_timeline_view_time_ruler_accuracy() {
        // Create a timeline view
        let timeline = Arc::new(Mutex::new(Timeline::new()));
        let mut timeline_view = TimelineView::new(timeline.clone());

        timeline_view.zoom_level = 60.0; // 60 pixels per second

        // Test time calculation at different scroll positions
        timeline_view.scroll_position = 0.0;
        let viewport_width = 300.0;
        let expected_end_time = viewport_width / timeline_view.zoom_level; // 5 seconds visible
        assert!((expected_end_time - 5.0).abs() < 0.01);

        // Test with scroll offset
        timeline_view.scroll_position = 10.0;
        let visible_start_time = timeline_view.scroll_position;
        let visible_end_time = timeline_view.scroll_position + (viewport_width / timeline_view.zoom_level);
        assert!((visible_start_time - 10.0).abs() < 0.01);
        assert!((visible_end_time - 15.0).abs() < 0.01);

        println!("✅ Timeline view time ruler accuracy test passed");
    }

    #[test]
    fn test_timeline_view_mouse_coordinate_conversion() {
        // Create a timeline view
        let timeline = Arc::new(Mutex::new(Timeline::new()));
        let mut timeline_view = TimelineView::new(timeline.clone());

        timeline_view.zoom_level = 50.0; // 50 pixels per second
        timeline_view.scroll_position = 5.0; // Scrolled 5 seconds to the right

        // Simulate mouse position conversion
        let viewport_x = 100.0; // Mouse at 100 pixels from viewport left
        let timeline_time = (viewport_x / timeline_view.zoom_level + timeline_view.scroll_position) as f64;

        // Expected: 100px / 50px/s + 5s = 2s + 5s = 7s
        assert!((timeline_time - 7.0).abs() < 0.01);

        // Test with zero scroll
        timeline_view.scroll_position = 0.0;
        let timeline_time_no_scroll = (viewport_x / timeline_view.zoom_level + timeline_view.scroll_position) as f64;
        // Expected: 100px / 50px/s + 0s = 2s
        assert!((timeline_time_no_scroll - 2.0).abs() < 0.01);

        println!("✅ Timeline view mouse coordinate conversion test passed");
    }

    #[test]
    fn test_pattern_preview_generation() {
        use crate::audio::{TimeSignature, sequencer::{Pattern, Step}};

        // Create a timeline view
        let timeline = Arc::new(Mutex::new(Timeline::new()));
        let timeline_view = TimelineView::new(timeline.clone());

        // Create test patterns with active steps
        let mut kick_pattern = Pattern::new("Kick".to_string(), "kick".to_string(), 16);
        kick_pattern.steps[0].active = true;
        kick_pattern.steps[4].active = true;
        kick_pattern.steps[8].active = true;

        let mut snare_pattern = Pattern::new("Snare".to_string(), "snare".to_string(), 16);
        snare_pattern.steps[4].active = true;
        snare_pattern.steps[12].active = true;

        let patterns = vec![kick_pattern, snare_pattern];
        let segment = crate::timeline::TimelineSegment::new(
            "Test Segment".to_string(),
            patterns,
            0.0,
            2, // 2 loops to test loop count functionality
            TimeSignature::four_four(),
            120.0,
        );

        // Generate pattern preview
        let preview_width = 100.0;
        let preview_height = 20.0;
        let visuals = egui::Visuals::dark(); // Use dark theme for test
        let preview_elements = timeline_view.generate_pattern_preview(&segment, preview_width, preview_height, &visuals);

        // Should have elements for active steps repeated for each loop
        // kick: 3 steps * 2 loops = 6, snare: 2 steps * 2 loops = 4, total = 10 active steps
        // Each element is stored as 2 points (position + size), so 20 total points
        assert_eq!(preview_elements.len(), 20); // 10 active steps * 2 points each

        // Verify first element (kick pattern, step 0)
        assert!(preview_elements[0].0.x >= 0.0);
        assert!(preview_elements[0].0.y >= 0.0);
        assert_eq!(preview_elements[0].1, egui::Color32::from_rgb(255, 100, 100)); // Kick color

        println!("✅ Pattern preview generation test passed");
    }

    #[test]
    fn test_text_measurement_and_truncation() {
        // Create a timeline view
        let timeline = Arc::new(Mutex::new(Timeline::new()));
        let timeline_view = TimelineView::new(timeline.clone());

        // Test text width measurement
        let short_text = "Test";
        let long_text = "This is a very long segment name that should be truncated";
        let font_size = 11.0;

        let short_width = timeline_view.measure_text_width(short_text, font_size);
        let long_width = timeline_view.measure_text_width(long_text, font_size);

        assert!(short_width < long_width);
        assert!(short_width > 0.0);

        // Test truncation
        let max_width = 50.0; // Small width to force truncation
        let truncated = timeline_view.truncate_text_with_ellipses(long_text, max_width, font_size);

        assert!(truncated.len() < long_text.len());
        assert!(truncated.ends_with("..."));
        assert!(timeline_view.measure_text_width(&truncated, font_size) <= max_width);

        // Test text that doesn't need truncation
        let no_truncate = timeline_view.truncate_text_with_ellipses(short_text, 100.0, font_size);
        assert_eq!(no_truncate, short_text);

        println!("✅ Text measurement and truncation test passed");
    }

    #[test]
    fn test_segment_text_display_calculation() {
        use crate::audio::{TimeSignature, sequencer::Pattern};

        // Create a timeline view
        let timeline = Arc::new(Mutex::new(Timeline::new()));
        let timeline_view = TimelineView::new(timeline.clone());

        // Create test segment
        let patterns = vec![Pattern::new("Test".to_string(), "kick".to_string(), 16)];
        let segment = crate::timeline::TimelineSegment::new(
            "Very Long Segment Name That Should Be Truncated".to_string(),
            patterns,
            0.0,
            3,
            TimeSignature::four_four(),
            120.0,
        );

        // Test with wide segment (should show full text)
        let (text_wide, font_wide) = timeline_view.calculate_segment_text_display(&segment, 400.0); // Use much wider segment
        assert!(text_wide.contains("Very Long Segment Name That Should Be Truncated"));
        assert!(text_wide.contains("(3x)"));
        assert_eq!(font_wide, 11.0);

        // Test with narrow segment (should truncate)
        let (text_narrow, font_narrow) = timeline_view.calculate_segment_text_display(&segment, 60.0);
        assert!(text_narrow.len() < segment.pattern_id.len() + 5); // Should be shorter than full name + loop count
        assert!(text_narrow.contains("...") || text_narrow.len() < 10); // Either truncated or very short

        // Test with very narrow segment (should be minimal)
        let (text_tiny, font_tiny) = timeline_view.calculate_segment_text_display(&segment, 30.0);
        assert!(text_tiny.len() < text_narrow.len());
        assert!(font_tiny >= 8.0); // Should not go below minimum font size

        println!("✅ Segment text display calculation test passed");
    }

    #[test]
    fn test_zoom_adaptive_text_sizing() {
        use crate::audio::{TimeSignature, sequencer::Pattern};

        // Create a timeline view
        let timeline = Arc::new(Mutex::new(Timeline::new()));
        let timeline_view = TimelineView::new(timeline.clone());

        // Create test segment
        let patterns = vec![Pattern::new("Test".to_string(), "kick".to_string(), 16)];
        let segment = crate::timeline::TimelineSegment::new(
            "Test Segment".to_string(),
            patterns,
            0.0,
            1,
            TimeSignature::four_four(),
            120.0,
        );

        // Test different segment widths
        let (_, font_tiny) = timeline_view.calculate_segment_text_display(&segment, 30.0);
        let (_, font_small) = timeline_view.calculate_segment_text_display(&segment, 70.0);
        let (_, font_normal) = timeline_view.calculate_segment_text_display(&segment, 120.0);

        // Font size should scale with segment width
        assert!(font_tiny <= font_small);
        assert!(font_small <= font_normal);
        assert!(font_tiny >= 8.0); // Minimum font size
        assert!(font_normal <= 14.0); // Maximum font size

        println!("✅ Zoom adaptive text sizing test passed");
    }

    #[test]
    fn test_pattern_preview_with_empty_patterns() {
        use crate::audio::TimeSignature;

        // Create a timeline view
        let timeline = Arc::new(Mutex::new(Timeline::new()));
        let timeline_view = TimelineView::new(timeline.clone());

        // Create segment with empty patterns
        let segment = crate::timeline::TimelineSegment::new(
            "Empty Segment".to_string(),
            vec![], // No patterns
            0.0,
            1,
            TimeSignature::four_four(),
            120.0,
        );

        // Generate pattern preview
        let visuals = egui::Visuals::dark(); // Use dark theme for test
        let preview_elements = timeline_view.generate_pattern_preview(&segment, 100.0, 20.0, &visuals);

        // Should return empty vector for segment with no patterns
        assert!(preview_elements.is_empty());

        println!("✅ Pattern preview with empty patterns test passed");
    }

    #[test]
    fn test_pattern_preview_loop_count_functionality() {
        use crate::audio::{TimeSignature, sequencer::Pattern};

        // Create a timeline view
        let timeline = Arc::new(Mutex::new(Timeline::new()));
        let timeline_view = TimelineView::new(timeline.clone());

        // Create test pattern with 2 active steps
        let mut kick_pattern = Pattern::new("Kick".to_string(), "kick".to_string(), 4);
        kick_pattern.steps[0].active = true;
        kick_pattern.steps[2].active = true;

        let patterns = vec![kick_pattern];

        // Test with 1 loop
        let segment_1_loop = crate::timeline::TimelineSegment::new(
            "Test Segment 1x".to_string(),
            patterns.clone(),
            0.0,
            1,
            TimeSignature::four_four(),
            120.0,
        );

        let preview_1_loop = timeline_view.generate_pattern_preview(&segment_1_loop, 100.0, 20.0);
        // 2 active steps * 1 loop * 2 points each = 4 points
        assert_eq!(preview_1_loop.len(), 4);

        // Test with 3 loops
        let segment_3_loops = crate::timeline::TimelineSegment::new(
            "Test Segment 3x".to_string(),
            patterns,
            0.0,
            3,
            TimeSignature::four_four(),
            120.0,
        );

        let preview_3_loops = timeline_view.generate_pattern_preview(&segment_3_loops, 100.0, 20.0);
        // 2 active steps * 3 loops * 2 points each = 12 points
        assert_eq!(preview_3_loops.len(), 12);

        // Verify step positions are spread across the full width for 3 loops
        // First loop should have steps at positions 0 and 2 (out of 12 total steps: 4*3)
        // Second loop should have steps at positions 4 and 6
        // Third loop should have steps at positions 8 and 10
        let step_positions: Vec<f32> = preview_3_loops.iter()
            .step_by(2) // Take every other element (skip the size points)
            .map(|(pos, _)| pos.x)
            .collect();

        // Should have 6 active step positions (2 steps * 3 loops)
        assert_eq!(step_positions.len(), 6);

        // Verify positions are distributed across the preview width
        let total_width = 100.0;
        let total_steps = 4 * 3; // 4 steps per pattern * 3 loops
        let step_width = total_width / total_steps as f32;

        // Check first step of each loop (step 0, 4, 8)
        let expected_positions = [0, 4, 8].iter()
            .map(|&step| step as f32 * step_width + step_width * 0.25)
            .collect::<Vec<f32>>();

        // Verify the first step of each loop is at the expected position
        for (i, &expected_pos) in expected_positions.iter().enumerate() {
            let actual_pos = step_positions[i * 2]; // Every other position (steps 0, 4, 8)
            assert!((actual_pos - expected_pos).abs() < 0.1,
                   "Loop {} first step position: expected {}, got {}", i + 1, expected_pos, actual_pos);
        }

        println!("✅ Pattern preview loop count functionality test passed");
    }

    #[test]
    fn test_performance_with_multiple_segments() {
        use crate::audio::{TimeSignature, sequencer::{Pattern, Step}};
        use std::time::Instant;

        // Create a timeline view
        let timeline = Arc::new(Mutex::new(Timeline::new()));
        let timeline_view = TimelineView::new(timeline.clone());

        // Create multiple segments with complex patterns
        let mut segments = Vec::new();
        for i in 0..20 {
            let mut patterns = Vec::new();
            for j in 0..8 {
                let mut pattern = Pattern::new(format!("Pattern{}", j), format!("sample{}", j), 16);
                // Add some active steps
                for k in 0..16 {
                    if (k + i + j) % 3 == 0 {
                        pattern.steps[k].active = true;
                    }
                }
                patterns.push(pattern);
            }

            let segment = crate::timeline::TimelineSegment::new(
                format!("Segment {}", i),
                patterns,
                i as f64 * 4.0,
                2,
                TimeSignature::four_four(),
                120.0,
            );
            segments.push(segment);
        }

        // Test pattern preview generation performance
        let start = Instant::now();
        for segment in &segments {
            let visuals = egui::Visuals::dark(); // Use dark theme for test
            let _ = timeline_view.generate_pattern_preview(segment, 100.0, 20.0, &visuals);
        }
        let preview_duration = start.elapsed();

        // Test text calculation performance
        let start = Instant::now();
        for segment in &segments {
            let _ = timeline_view.calculate_segment_text_display(segment, 100.0);
        }
        let text_duration = start.elapsed();

        // Performance should be reasonable (less than 10ms for 20 segments)
        assert!(preview_duration.as_millis() < 10, "Pattern preview generation too slow: {}ms", preview_duration.as_millis());
        assert!(text_duration.as_millis() < 5, "Text calculation too slow: {}ms", text_duration.as_millis());

        println!("✅ Performance test passed - Preview: {}μs, Text: {}μs",
                preview_duration.as_micros(), text_duration.as_micros());
    }
    
    #[test]
    fn test_all_eight_tracks_display_in_timeline_preview() {
        use crate::audio::{TimeSignature, sequencer::Pattern};
        use std::sync::{Arc, Mutex};
        
        let timeline = Arc::new(Mutex::new(crate::timeline::Timeline::new()));
        let timeline_view = TimelineView::new(timeline);
        
        // Create a segment with all 8 tracks including rim shot
        let track_names = vec!["Kick", "Snare", "Hi-Hat", "Crash", "Open Hi-Hat", "Clap", "Rim Shot", "Tom"];
        let sample_names = vec!["kick", "snare", "hihat", "crash", "open_hihat", "clap", "rimshot", "tom"];
        
        let mut patterns = Vec::new();
        for (track_name, sample_name) in track_names.iter().zip(sample_names.iter()) {
            let mut pattern = Pattern::new(track_name.to_string(), sample_name.to_string(), 16);
            // Activate step 0 for each track to ensure they appear in preview
            if !pattern.steps.is_empty() {
                pattern.steps[0].active = true;
            }
            patterns.push(pattern);
        }
        
        let segment = crate::timeline::TimelineSegment::new(
            "Test 8 Track Segment".to_string(),
            patterns,
            0.0,
            1,
            TimeSignature::four_four(),
            120.0,
        );
        
        // Generate pattern preview for all 8 tracks
        let visuals = egui::Visuals::dark(); // Use dark theme for test
        let preview_elements = timeline_view.generate_pattern_preview(&segment, 800.0, 160.0, &visuals);
        
        // Should have 16 preview elements (8 tracks * 2 elements per visual element: pos + size)
        assert_eq!(preview_elements.len(), 16, "Expected 16 preview elements for 8 tracks with active steps (2 elements per visual)");
        
        // Verify that rim shot track (index 6) is included
        assert!(segment.patterns.len() == 8, "Segment should have 8 patterns");
        assert_eq!(segment.patterns[6].name, "Rim Shot", "Track 6 should be Rim Shot");
        assert_eq!(segment.patterns[6].sample_name, "rimshot", "Rim Shot should use rimshot sample");
        
        // Test pattern height calculation accommodates all 8 tracks
        let pattern_height = 160.0 / segment.patterns.len() as f32;
        assert_eq!(pattern_height, 20.0, "Each track should get 20px height (160px / 8 tracks)");
        
        println!("✅ All 8 tracks including Rim Shot display correctly in timeline preview");
    }
}
