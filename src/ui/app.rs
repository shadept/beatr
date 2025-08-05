use super::components::{
    PatternGrid, TempoControl, TimeSignatureControl, TimelineView, TransportControls,
};
use crate::audio::AudioEngine;
use crate::project::Project;
use crate::timeline::Timeline;
use eframe::egui;
use std::sync::{Arc, Mutex};
use std::path::PathBuf;

pub struct DrumComposerApp {
    audio_engine: Option<AudioEngine>,
    error_message: Option<String>,
    tempo: f32,
    custom_loop_length_text: String,
    custom_time_sig_numerator: String,
    custom_time_sig_denominator: String,
    time_sig_validation_error: Option<String>,
    timeline: Arc<Mutex<Timeline>>,
    timeline_view: Option<TimelineView>,
    // Project management
    current_project: Project,
    current_project_path: Option<PathBuf>,
    project_modified: bool,
}

impl DrumComposerApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let mut app = DrumComposerApp {
            audio_engine: None,
            error_message: None,
            tempo: 120.0,
            custom_loop_length_text: "16".to_string(),
            custom_time_sig_numerator: "4".to_string(),
            custom_time_sig_denominator: "4".to_string(),
            time_sig_validation_error: None,
            timeline: Arc::new(Mutex::new(Timeline::new())), // Temporary, will be replaced
            timeline_view: None,
            current_project: Project::new("New Project".to_string()),
            current_project_path: None,
            project_modified: false,
        };

        // Initialize audio engine
        match AudioEngine::new() {
            Ok(engine) => {
                // Samples are now loaded automatically in AudioEngine::new()

                // Get the timeline from the audio engine 
                app.timeline = engine.timeline();
                
                // Sync the project timeline with the audio engine timeline
                app.sync_project_to_audio_timeline();
                
                // Create a default timeline segment for new projects
                if app.current_project.timeline.segments.is_empty() {
                    use crate::audio::{TimeSignature, sequencer::Pattern};
                    use crate::timeline::TimelineSegment;
                    
                    let pattern_names = vec!["Kick", "Snare", "Hi-Hat", "Crash", "Open Hi-Hat", "Clap", "Rim Shot", "Tom"];
                    let pattern_samples = vec!["kick", "snare", "hihat", "crash", "open_hihat", "clap", "rimshot", "tom"];
                    
                    let patterns: Vec<Pattern> = pattern_names.iter().zip(pattern_samples.iter()).map(|(name, sample)| {
                        Pattern::new(name.to_string(), sample.to_string(), 16)
                    }).collect();
                    
                    let default_segment = TimelineSegment::new(
                        "Default Pattern".to_string(),
                        patterns,
                        0.0,
                        1,
                        TimeSignature::four_four(),
                        120.0,
                    );
                    
                    app.current_project.timeline.add_segment(default_segment);
                    app.sync_project_to_audio_timeline();
                }
                
                app.timeline_view = Some(TimelineView::new(app.timeline.clone()));

                app.audio_engine = Some(engine);
            }
            Err(e) => {
                app.error_message = Some(format!("Failed to initialize audio: {}", e));
            }
        }

        app
    }

    // Project management methods
    fn sync_project_to_audio_timeline(&mut self) {
        if let Ok(mut audio_timeline) = self.timeline.lock() {
            *audio_timeline = self.current_project.timeline.clone();
            // Sync UI tempo with timeline average BPM
            self.tempo = audio_timeline.get_average_bpm();
        }
    }

    fn sync_audio_timeline_to_project(&mut self) {
        if let Ok(audio_timeline) = self.timeline.lock() {
            self.current_project.timeline = audio_timeline.clone();
            self.project_modified = true;
        }
    }

    fn save_project(&mut self) {
        self.sync_audio_timeline_to_project();
        
        if let Some(path) = &self.current_project_path {
            match self.current_project.save_to_file(path) {
                Ok(()) => {
                    self.project_modified = false;
                    self.error_message = None;
                }
                Err(e) => {
                    self.error_message = Some(format!("Failed to save project: {}", e));
                }
            }
        } else {
            self.save_project_as();
        }
    }

    fn save_project_as(&mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("Beatr Project", &["beatr"])
                .set_file_name(&format!("{}.beatr", self.current_project.metadata.name))
                .save_file()
            {
                self.current_project_path = Some(path.clone());
                match self.current_project.save_to_file(&path) {
                    Ok(()) => {
                        self.project_modified = false;
                        self.error_message = None;
                    }
                    Err(e) => {
                        self.error_message = Some(format!("Failed to save project: {}", e));
                    }
                }
            }
        }
    }

    fn load_project(&mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("Beatr Project", &["beatr"])
                .pick_file()
            {
                match Project::load_from_file(&path) {
                    Ok(project) => {
                        match project.validate() {
                            Ok(()) => {
                                self.current_project = project;
                                self.current_project_path = Some(path);
                                self.project_modified = false;
                                self.error_message = None;
                                
                                // Sync the loaded project to the audio timeline
                                self.sync_project_to_audio_timeline();
                                
                                // Update UI values from project
                                self.tempo = self.current_project.global_bpm;
                            }
                            Err(e) => {
                                self.error_message = Some(format!("Invalid project file: {}", e));
                            }
                        }
                    }
                    Err(e) => {
                        self.error_message = Some(format!("Failed to load project: {}", e));
                    }
                }
            }
        }
    }

    fn new_project(&mut self) {
        if self.project_modified {
            // In a real app, you'd show a "Save changes?" dialog here
            // For now, we'll just create a new project
        }
        
        self.current_project = Project::new("New Project".to_string());
        self.current_project_path = None;
        self.project_modified = false;
        self.error_message = None;
        self.tempo = self.current_project.global_bpm;
        
        // Clear the audio timeline
        if let Ok(mut audio_timeline) = self.timeline.lock() {
            *audio_timeline = Timeline::new();
        }
    }

    fn get_window_title(&self) -> String {
        let project_name = &self.current_project.metadata.name;
        let modified_indicator = if self.project_modified { "*" } else { "" };
        format!("Beatr - {}{}", project_name, modified_indicator)
    }
}

impl eframe::App for DrumComposerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Set a dark theme for better visual contrast with the sequencer
        ctx.set_visuals(egui::Visuals::dark());

        // Update window title
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(self.get_window_title()));

        // Menu bar
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("New Project").clicked() {
                        self.new_project();
                        ui.close_menu();
                    }
                    
                    if ui.button("Open Project...").clicked() {
                        self.load_project();
                        ui.close_menu();
                    }
                    
                    ui.separator();
                    
                    if ui.button("Save Project").clicked() {
                        self.save_project();
                        ui.close_menu();
                    }
                    
                    if ui.button("Save Project As...").clicked() {
                        self.save_project_as();
                        ui.close_menu();
                    }
                    
                    ui.separator();
                    
                    if ui.button("Project Info...").clicked() {
                        // TODO: Show project info dialog
                        ui.close_menu();
                    }
                });
                
                ui.menu_button("Edit", |ui| {
                    // TODO: Add edit menu items (undo, redo, etc.)
                    ui.label("Edit operations (coming soon)");
                });
                
                ui.menu_button("Help", |ui| {
                    if ui.button("About").clicked() {
                        // TODO: Show about dialog
                        ui.close_menu();
                    }
                });
                
                // Show project status
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if self.project_modified {
                        ui.colored_label(egui::Color32::YELLOW, "● Unsaved changes");
                    } else {
                        ui.colored_label(egui::Color32::GREEN, "● Saved");
                    }
                });
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            // Header with improved styling
            egui::Frame::none()
                .fill(egui::Color32::from_gray(20))
                .inner_margin(egui::Margin::symmetric(16.0, 12.0))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.heading("🥁 Beatr");
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label("Drum Track Composer");
                        });
                    });
                });

            ui.add_space(8.0);

            if let Some(ref error) = self.error_message {
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(60, 20, 20))
                    .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(120, 40, 40)))
                    .inner_margin(egui::Margin::same(12.0))
                    .show(ui, |ui| {
                        ui.colored_label(egui::Color32::from_rgb(255, 100, 100),
                                       format!("⚠ Error: {}", error));
                    });
                ui.add_space(8.0);
                return;
            }

            if let Some(ref _audio_engine) = self.audio_engine {

                // Transport controls in a dedicated panel
                egui::Frame::none()
                    .fill(egui::Color32::from_gray(30))
                    .stroke(egui::Stroke::new(1.0, egui::Color32::from_gray(50)))
                    .inner_margin(egui::Margin::symmetric(16.0, 12.0))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label("Transport:");
                            ui.add_space(8.0);

                            // Check if timeline has segments to determine playback mode
                            let has_timeline_segments = {
                                if let Ok(timeline) = self.timeline.lock() {
                                    !timeline.segments.is_empty()
                                } else {
                                    false
                                }
                            };

                            if has_timeline_segments {
                                // Timeline transport controls
                                let (is_timeline_playing, timeline_position) = {
                                    if let Ok(timeline) = self.timeline.lock() {
                                        (timeline.is_playing(), timeline.current_position)
                                    } else {
                                        (false, 0.0)
                                    }
                                };

                                if ui.button(if is_timeline_playing { "⏸ Pause" } else { "▶ Play Timeline" }).clicked() {
                                    if let Ok(mut timeline) = self.timeline.lock() {
                                        if is_timeline_playing {
                                            timeline.pause();
                                        } else {
                                            timeline.play();
                                        }
                                        // Playback state changes don't mark project as modified
                                    }
                                }

                                if ui.button("⏹ Stop").clicked() {
                                    if let Ok(mut timeline) = self.timeline.lock() {
                                        timeline.stop();
                                        // Playback state changes don't mark project as modified
                                    }
                                }

                                // Show timeline position
                                ui.label(format!("Position: {:.1}s", timeline_position));
                            } else {
                                // Regular sequencer transport controls
                                if TransportControls::show(ui, &self.timeline) {
                                    // Transport state changed
                                }
                            }

                            ui.separator();
                            ui.add_space(8.0);

                            ui.label("Default Tempo:");
                            ui.add_space(4.0);

                            let tempo_changed = TempoControl::show(ui, &mut self.tempo);
                            
                            if tempo_changed {
                                // Show option to apply to all segments
                                ui.add_space(4.0);
                                if ui.button("Apply to All Segments").clicked() {
                                    if let Ok(mut timeline) = self.timeline.lock() {
                                        timeline.set_global_bpm(self.tempo);
                                    }
                                    self.project_modified = true;
                                }
                            }

                            ui.separator();
                            ui.add_space(8.0);

                            ui.label("Loop: Timeline Segments");
                            ui.separator();
                            let selected_segment_id = if let Some(ref timeline_view) = self.timeline_view {
                                timeline_view.get_selected_segment_id()
                            } else {
                                None
                            };
                            let time_sig_changed = TimeSignatureControl::show(
                                ui, 
                                &self.timeline, 
                                selected_segment_id.as_deref(), 
                                &mut self.custom_time_sig_numerator, 
                                &mut self.custom_time_sig_denominator,
                                &mut self.time_sig_validation_error
                            );
                            
                            if time_sig_changed {
                                self.project_modified = true;
                            }
                            
                            ui.add_space(4.0);
                            // Status indicators on the right
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.label("🎼 Timeline Mode");
                            });
                        });
                    });

                ui.add_space(12.0);

                // Timeline view first - this updates sequencer patterns based on selection
                let mut timeline_modified = false;
                if let Some(ref mut timeline_view) = self.timeline_view {
                    egui::Frame::none()
                        .fill(egui::Color32::from_gray(30))
                        .stroke(egui::Stroke::new(1.0, egui::Color32::from_gray(50)))
                        .inner_margin(egui::Margin::symmetric(12.0, 8.0))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.heading("Timeline");
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    if let Ok(timeline) = self.timeline.lock() {
                                        let duration = timeline.total_duration();
                                        ui.label(format!("Duration: {:.1}s", duration));
                                    }
                                });
                            });
                            ui.add_space(4.0);
                            timeline_view.show(ui, &self.timeline, self.tempo);
                            timeline_modified = true; // Assume timeline was modified
                        });
                }
                
                // Sync timeline changes with project after UI interaction
                if timeline_modified {
                    self.sync_audio_timeline_to_project();
                }

                ui.add_space(12.0);

                // Pattern grid with improved container - shown after timeline view updates sequencer
                let selected_segment_name = if let Some(ref timeline_view) = self.timeline_view {
                    if let Some(selected_id) = timeline_view.get_selected_segment_id() {
                        if let Ok(timeline) = self.timeline.lock() {
                            timeline.get_segment(&selected_id).map(|s| s.pattern_id.clone())
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                };

                egui::ScrollArea::vertical()
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        if let Some(ref segment_name) = selected_segment_name {
                            ui.horizontal(|ui| {
                                ui.label("Editing timeline segment:");
                                ui.colored_label(egui::Color32::from_rgb(100, 140, 220), segment_name);
                            });
                            ui.add_space(4.0);
                        }
                        let selected_segment_id = if let Some(ref timeline_view) = self.timeline_view {
                            timeline_view.get_selected_segment_id()
                        } else {
                            None
                        };
                        PatternGrid::show(ui, &self.timeline, selected_segment_id.as_deref());
                    });

                // No sync needed - patterns are stored directly in timeline segments

                ui.add_space(8.0);

                // Footer with help text
                egui::Frame::none()
                    .fill(egui::Color32::from_gray(20))
                    .inner_margin(egui::Margin::symmetric(16.0, 8.0))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.small("💡 Click step buttons to create patterns • Numbers show measure positions • Clear to reset tracks");
                        });
                    });
            } else {
                // Loading state with better styling
                ui.vertical_centered(|ui| {
                    ui.add_space(100.0);
                    ui.spinner();
                    ui.add_space(16.0);
                    ui.label("Initializing audio engine...");
                });
            }
        });

        // Request repaint for real-time updates
        ctx.request_repaint();
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, "tempo", &self.tempo);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        // Stop audio before closing
        if let Some(ref audio_engine) = self.audio_engine {
            if let Ok(mut timeline) = audio_engine.timeline().try_lock() {
                timeline.stop();
            }
        }
    }
}
