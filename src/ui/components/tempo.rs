use eframe::egui;

pub struct TempoControl;

impl TempoControl {
    pub fn show(ui: &mut egui::Ui, tempo: &mut f32) -> bool {
        let mut changed = false;
        
        ui.horizontal(|ui| {
            ui.label("BPM:");
            
            if ui.add(egui::DragValue::new(tempo)
                .range(60.0..=200.0)
                .speed(1.0)
                .suffix(" BPM"))
                .changed() 
            {
                changed = true;
            }
            
            // Preset tempo buttons
            if ui.button("80").clicked() {
                *tempo = 80.0;
                changed = true;
            }
            if ui.button("120").clicked() {
                *tempo = 120.0;
                changed = true;
            }
            if ui.button("140").clicked() {
                *tempo = 140.0;
                changed = true;
            }
            if ui.button("160").clicked() {
                *tempo = 160.0;
                changed = true;
            }
        });
        
        changed
    }
}