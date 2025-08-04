use eframe::egui;
use std::sync::{Arc, Mutex};

use crate::audio::{Sequencer, TimeSignature};

pub struct TimeSignatureControl;

impl TimeSignatureControl {
    pub fn show(ui: &mut egui::Ui, sequencer: &Arc<Mutex<Sequencer>>, custom_numerator: &mut String, custom_denominator: &mut String) -> bool {
        let mut changed = false;
        let current_time_signature = {
            let sequencer_lock = sequencer.lock().unwrap();
            sequencer_lock.get_time_signature()
        };

        ui.horizontal(|ui| {
            ui.label("Time Signature:");
            ui.add_space(4.0);

            // Preset buttons for common time signatures
            let presets = [
                ("4/4", TimeSignature::four_four()),
                ("3/4", TimeSignature::three_four()),
                ("5/4", TimeSignature::five_four()),
                ("6/8", TimeSignature::six_eight()),
                ("7/8", TimeSignature::seven_eight()),
                ("9/8", TimeSignature::nine_eight()),
                ("12/8", TimeSignature::twelve_eight()),
            ];

            for (label, preset_ts) in &presets {
                let is_selected = current_time_signature == *preset_ts;
                let button = egui::Button::new(*label)
                    .fill(if is_selected {
                        egui::Color32::from_rgb(0, 150, 0) // Green for selected
                    } else {
                        egui::Color32::from_gray(60) // Dark gray for unselected
                    });

                if ui.add(button).clicked() {
                    if let Ok(mut seq) = sequencer.try_lock() {
                        seq.set_time_signature(*preset_ts);
                        changed = true;
                    }
                }
            }

            ui.separator();

            // Custom time signature input
            ui.label("Custom:");
            ui.add_space(2.0);

            // Numerator input
            let numerator_edit = egui::TextEdit::singleline(custom_numerator)
                .desired_width(25.0)
                .hint_text("2");
            let numerator_response = ui.add(numerator_edit);

            ui.label("/");

            // Denominator input
            let denominator_edit = egui::TextEdit::singleline(custom_denominator)
                .desired_width(25.0)
                .hint_text("2");
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

            // Process custom input on Enter or lost focus
            if (numerator_response.lost_focus() || denominator_response.lost_focus()) ||
               ((numerator_response.has_focus() || denominator_response.has_focus()) &&
                ui.input(|i| i.key_pressed(egui::Key::Enter))) {

                if let (Ok(num), Ok(den)) = (custom_numerator.parse::<u8>(), custom_denominator.parse::<u8>()) {
                    match TimeSignature::new(num, den) {
                        Ok(new_ts) => {
                            if let Ok(mut seq) = sequencer.try_lock() {
                                seq.set_time_signature(new_ts);
                                changed = true;
                            }
                        }
                        Err(_) => {
                            // Reset to current values if invalid
                            *custom_numerator = current_time_signature.numerator.to_string();
                            *custom_denominator = current_time_signature.denominator.to_string();
                        }
                    }
                } else if numerator_response.lost_focus() || denominator_response.lost_focus() {
                    // Reset to current values if parse fails and focus is lost
                    *custom_numerator = current_time_signature.numerator.to_string();
                    *custom_denominator = current_time_signature.denominator.to_string();
                }
            }

            // Display current time signature info
            ui.add_space(8.0);
            ui.colored_label(
                egui::Color32::LIGHT_BLUE,
                format!("({})", current_time_signature.display_string())
            );

            // Show optimal loop length hint
            let optimal_length = current_time_signature.optimal_loop_length(4);
            let current_loop_length = {
                let sequencer_lock = sequencer.lock().unwrap();
                sequencer_lock.get_loop_length()
            };

            if optimal_length != current_loop_length {
                ui.add_space(4.0);
                ui.colored_label(
                    egui::Color32::YELLOW,
                    format!("💡 Suggested: {} steps", optimal_length)
                );
            }
        });

        changed
    }
}
