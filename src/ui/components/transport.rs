use eframe::egui;
use std::sync::{Arc, Mutex};

use crate::timeline::Timeline;

pub struct TransportControls;

impl TransportControls {
    pub fn show(ui: &mut egui::Ui, timeline: &Arc<Mutex<Timeline>>) -> bool {
        let mut state_changed = false;

        ui.horizontal(|ui| {
            let is_playing = timeline.lock().unwrap().is_playing();

            if is_playing {
                if ui.button("⏸ Pause").clicked() {
                    timeline.lock().unwrap().pause();
                    state_changed = true;
                }
            } else {
                if ui.button("▶ Play").clicked() {
                    timeline.lock().unwrap().play();
                    state_changed = true;
                }
            }

            if ui.button("⏹ Stop").clicked() {
                timeline.lock().unwrap().stop();
                state_changed = true;
            }
        });

        state_changed
    }
}
