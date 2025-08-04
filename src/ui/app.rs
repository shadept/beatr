use super::components::{
    PatternGrid, TempoControl, TimelineView, TransportControls,
};
use crate::audio::AudioEngine;
use crate::timeline::Timeline;
use eframe::egui;
use std::sync::{Arc, Mutex};

pub struct DrumComposerApp {
    audio_engine: Option<AudioEngine>,
    error_message: Option<String>,
    tempo: f32,
    custom_loop_length_text: String,
    custom_time_sig_numerator: String,
    custom_time_sig_denominator: String,
    timeline: Arc<Mutex<Timeline>>,
    timeline_view: Option<TimelineView>,
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
            timeline: Arc::new(Mutex::new(Timeline::new())), // Temporary, will be replaced
            timeline_view: None,
        };

        // Initialize audio engine
        match AudioEngine::new() {
            Ok(engine) => {
                // Samples are now loaded automatically in AudioEngine::new()

                // Get the timeline from the audio engine and create the timeline view
                app.timeline = engine.timeline();
                
                // Create a default timeline segment for pattern editing
                {
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
                    
                    let mut timeline = app.timeline.lock().unwrap();
                    timeline.add_segment(default_segment);
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
}

impl eframe::App for DrumComposerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Set a dark theme for better visual contrast with the sequencer
        ctx.set_visuals(egui::Visuals::dark());

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
                                    }
                                }

                                if ui.button("⏹ Stop").clicked() {
                                    if let Ok(mut timeline) = self.timeline.lock() {
                                        timeline.stop();
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

                            ui.label("Tempo:");
                            ui.add_space(4.0);

                            if TempoControl::show(ui, &mut self.tempo) {
                                // Update sequencer tempo
                                // Timeline mode - tempo controlled by segments
                            }

                            ui.separator();
                            ui.add_space(8.0);

                            ui.label("Loop: Timeline Segments");
                            ui.separator();
                            ui.add_space(8.0);
                            ui.label("Time Sig: Per Segment");

                            // Status indicators on the right
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.label("🎼 Timeline Mode");
                            });
                        });
                    });

                ui.add_space(12.0);

                // Timeline view first - this updates sequencer patterns based on selection
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
                            timeline_view.show(ui, &self.timeline);
                        });
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
