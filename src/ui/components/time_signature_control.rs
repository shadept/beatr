use eframe::egui;
use std::sync::{Arc, Mutex};

use crate::audio::TimeSignature;
use crate::timeline::Timeline;

pub struct TimeSignatureControl;

impl TimeSignatureControl {
    pub fn show(ui: &mut egui::Ui, timeline: &Arc<Mutex<Timeline>>, selected_segment_id: Option<&str>, custom_numerator: &mut String, custom_denominator: &mut String, validation_error: &mut Option<String>) -> bool {
        let mut changed = false;
        let current_time_signature = {
            let timeline_lock = timeline.lock().unwrap();
            // Get time signature from the selected segment, or default to 4/4
            if let Some(segment_id) = selected_segment_id {
                timeline_lock.get_segment(segment_id)
                    .map(|segment| segment.time_signature)
                    .unwrap_or_else(|| TimeSignature::four_four())
            } else {
                TimeSignature::four_four()
            }
        };

        // Multi-row compact layout for time signature controls
        ui.vertical(|ui| {
            // Row 1: Common presets (4 most used)
            ui.horizontal(|ui| {
                let common_presets = [
                    ("4/4", TimeSignature::four_four()),
                    ("3/4", TimeSignature::three_four()),
                    ("6/8", TimeSignature::six_eight()),
                    ("7/8", TimeSignature::seven_eight()),
                ];

                for (label, preset_ts) in &common_presets {
                    let is_selected = current_time_signature == *preset_ts;
                    let button = egui::Button::new(*label)
                        .fill(if is_selected {
                            egui::Color32::from_rgb(0, 150, 0)
                        } else {
                            egui::Color32::from_gray(60)
                        });

                    if ui.add(button).clicked() {
                        if let Ok(mut timeline_lock) = timeline.try_lock() {
                            if let Some(segment_id) = selected_segment_id {
                                if let Some(segment) = timeline_lock.get_segment_mut(segment_id) {
                                    segment.time_signature = *preset_ts;
                                    changed = true;
                                }
                            }
                        }
                    }
                }

                ui.separator();

                // Custom input in same row
                ui.add_space(2.0);
                let numerator_edit = egui::TextEdit::singleline(custom_numerator)
                    .desired_width(20.0)
                    .hint_text("4");
                let numerator_response = ui.add(numerator_edit);

                ui.label("/");

                let denominator_edit = egui::TextEdit::singleline(custom_denominator)
                    .desired_width(20.0)
                    .hint_text("4");
                let denominator_response = ui.add(denominator_edit);

                // Update custom fields when time signature changes externally
                if !numerator_response.has_focus() && !denominator_response.has_focus() {
                    let expected_num = current_time_signature.numerator.to_string();
                    let expected_den = current_time_signature.denominator.to_string();
                    if custom_numerator != &expected_num || custom_denominator != &expected_den {
                        *custom_numerator = expected_num;
                        *custom_denominator = expected_den;
                    }
                }

                // Process custom input
                if (numerator_response.lost_focus() || denominator_response.lost_focus()) ||
                   ((numerator_response.has_focus() || denominator_response.has_focus()) &&
                    ui.input(|i| i.key_pressed(egui::Key::Enter))) {

                    if let (Ok(num), Ok(den)) = (custom_numerator.parse::<u8>(), custom_denominator.parse::<u8>()) {
                        match TimeSignature::new(num, den) {
                            Ok(new_ts) => {
                                if let Ok(mut timeline_lock) = timeline.try_lock() {
                                    if let Some(segment_id) = selected_segment_id {
                                        if let Some(segment) = timeline_lock.get_segment_mut(segment_id) {
                                            segment.time_signature = new_ts;
                                            changed = true;
                                            *validation_error = None;
                                        }
                                    }
                                }
                            }
                            Err(error_msg) => {
                                *validation_error = Some(error_msg);
                                *custom_numerator = current_time_signature.numerator.to_string();
                                *custom_denominator = current_time_signature.denominator.to_string();
                            }
                        }
                    } else if numerator_response.lost_focus() || denominator_response.lost_focus() {
                        *validation_error = Some("Invalid number format.".to_string());
                        *custom_numerator = current_time_signature.numerator.to_string();
                        *custom_denominator = current_time_signature.denominator.to_string();
                    }
                }

                // Compact current display
                ui.add_space(4.0);
                ui.small(format!("({})", current_time_signature.display_string()));
            });

            // Row 2: Additional presets and info (collapsible)
            ui.collapsing("More Options", |ui| {
                ui.horizontal(|ui| {
                    let more_presets = [
                        ("5/4", TimeSignature::five_four()),
                        ("9/8", TimeSignature::nine_eight()),
                        ("12/8", TimeSignature::twelve_eight()),
                    ];

                    for (label, preset_ts) in &more_presets {
                        let is_selected = current_time_signature == *preset_ts;
                        let button = egui::Button::new(*label)
                            .fill(if is_selected {
                                egui::Color32::from_rgb(0, 150, 0)
                            } else {
                                egui::Color32::from_gray(60)
                            });

                        if ui.add(button).clicked() {
                            if let Ok(mut timeline_lock) = timeline.try_lock() {
                                if let Some(segment_id) = selected_segment_id {
                                    if let Some(segment) = timeline_lock.get_segment_mut(segment_id) {
                                        segment.time_signature = *preset_ts;
                                        changed = true;
                                    }
                                }
                            }
                        }
                    }

                    // Show optimal loop length hint
                    let optimal_length = current_time_signature.optimal_loop_length(4);
                    let current_loop_length = {
                        let timeline_lock = timeline.lock().unwrap();
                        if let Some(segment_id) = selected_segment_id {
                            timeline_lock.get_segment(segment_id)
                                .map(|segment| segment.patterns.get(0).map(|pattern| pattern.steps.len()).unwrap_or(16))
                                .unwrap_or(16)
                        } else {
                            16
                        }
                    };

                    if optimal_length != current_loop_length {
                        ui.add_space(4.0);
                        ui.small(format!("💡 Suggested: {} steps", optimal_length));
                    }
                });
            });

            // Display validation error if present
            if let Some(error_msg) = validation_error {
                ui.colored_label(egui::Color32::RED, format!("⚠️ {}", error_msg));
            }
        });

        changed
    }
}
