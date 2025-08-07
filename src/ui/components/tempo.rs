use eframe::egui;

pub struct TempoControl;

impl TempoControl {
    pub fn show(ui: &mut egui::Ui, tempo: &mut f32) -> bool {
        let mut changed = false;

        ui.horizontal(|ui| {
            // Compact BPM input
            if ui
                .add(
                    egui::DragValue::new(tempo)
                        .range(60.0..=200.0)
                        .speed(1.0)
                        .prefix("♩ ")
                        .suffix(" BPM")
                        .min_decimals(0)
                        .max_decimals(0),
                )
                .changed()
            {
                changed = true;
            }

            ui.add_space(6.0);

            // Compact preset tempo buttons - flattened layout
            if ui.small_button("80").clicked() {
                *tempo = 80.0;
                changed = true;
            }
            if ui.small_button("120").clicked() {
                *tempo = 120.0;
                changed = true;
            }
            if ui.small_button("140").clicked() {
                *tempo = 140.0;
                changed = true;
            }
            if ui.small_button("160").clicked() {
                *tempo = 160.0;
                changed = true;
            }
        });

        changed
    }
}
