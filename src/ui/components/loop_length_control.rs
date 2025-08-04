use eframe::egui;
use std::sync::{Arc, Mutex};

use crate::audio::Sequencer;

pub struct LoopLengthControl;

impl LoopLengthControl {
    pub fn show(ui: &mut egui::Ui, sequencer: &Arc<Mutex<Sequencer>>, custom_text: &mut String) -> bool {
        let mut changed = false;
        let current_length = {
            let sequencer_lock = sequencer.lock().unwrap();
            sequencer_lock.get_loop_length()
        };

        ui.horizontal(|ui| {
            ui.label("Loop Length:");
            ui.add_space(4.0);

            // Preset buttons for common lengths
            let presets = [4, 8, 16, 32];
            for &preset in &presets {
                let button = egui::Button::new(format!("{}", preset))
                    .fill(if current_length == preset {
                        egui::Color32::from_rgb(0, 150, 0) // Green for selected
                    } else {
                        egui::Color32::from_gray(60) // Dark gray for unselected
                    });

                if ui.add(button).clicked() {
                    if let Ok(mut seq) = sequencer.try_lock() {
                        seq.set_loop_length(preset);
                        changed = true;
                    }
                }
            }

            ui.separator();

            // Custom length input
            ui.label("Custom:");
            ui.add_space(2.0);
            
            let text_edit = egui::TextEdit::singleline(custom_text)
                .desired_width(40.0)
                .hint_text("1-64");

            let text_response = ui.add(text_edit);
            
            // Only reset text when NOT actively typing (user experience priority)
            if !text_response.has_focus() {
                // Sync with sequencer value when field is not focused
                if custom_text.parse::<usize>().unwrap_or(0) != current_length {
                    *custom_text = current_length.to_string();
                }
            }
            
            // Process input on Enter key or lost focus
            if text_response.lost_focus() || (text_response.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))) {
                if let Ok(custom_length) = custom_text.parse::<usize>() {
                    if custom_length >= 1 && custom_length <= 64 {
                        if let Ok(mut seq) = sequencer.try_lock() {
                            seq.set_loop_length(custom_length);
                            changed = true;
                        }
                    } else {
                        // Reset to current value if out of range
                        *custom_text = current_length.to_string();
                    }
                } else if text_response.lost_focus() {
                    // Only reset on lost focus, not while typing
                    *custom_text = current_length.to_string();
                }
            }

            // Display current length
            ui.add_space(8.0);
            ui.colored_label(
                egui::Color32::LIGHT_BLUE,
                format!("({} steps)", current_length)
            );
        });

        changed
    }
}