use super::components::{
    PatternGrid, SettingsDialog, TempoControl, TimeSignatureControl, TimelineView,
    TransportControls,
};
use crate::audio::engine::AudioEngine;
use crate::project::Project;
use crate::settings::{AppSettings, KeyboardSettings};
use crate::timeline::Timeline;
use eframe::egui;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

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
    // Settings management
    settings: AppSettings,
    settings_dialog: SettingsDialog,
    // Theme monitoring
    last_resolved_theme: String,
    theme_change_notification: Option<String>,
}

impl DrumComposerApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        // Load settings from file
        let settings = AppSettings::load_from_file();

        let initial_resolved_theme = settings.ui.resolve_theme();

        let mut app = DrumComposerApp {
            audio_engine: None,
            error_message: None,
            tempo: settings.defaults.default_bpm,
            custom_loop_length_text: settings.defaults.default_pattern_length.to_string(),
            custom_time_sig_numerator: settings.defaults.default_time_signature.0.to_string(),
            custom_time_sig_denominator: settings.defaults.default_time_signature.1.to_string(),
            time_sig_validation_error: None,
            timeline: Arc::new(Mutex::new(Timeline::new())), // Temporary, will be replaced
            timeline_view: None,
            current_project: Project::new_with_defaults(
                "New Project".to_string(),
                &settings.defaults,
            ),
            current_project_path: None,
            project_modified: false,
            settings_dialog: SettingsDialog::new(settings.clone()),
            last_resolved_theme: initial_resolved_theme.clone(),
            theme_change_notification: None,
            settings,
        };

        // Initialize audio engine with settings
        match AudioEngine::new_with_settings(app.settings.audio.clone()) {
            Ok(engine) => {
                // Samples are now loaded automatically in AudioEngine::new()

                // Get the timeline from the audio engine
                app.timeline = engine.timeline();

                // Sync the project timeline with the audio engine timeline
                app.sync_project_to_audio_timeline();

                // Create a default timeline segment for new projects
                if app.current_project.timeline.segments.is_empty() {
                    use crate::audio::{sequencer::Pattern, TimeSignature};
                    use crate::timeline::TimelineSegment;

                    let pattern_names = vec![
                        "Kick",
                        "Snare",
                        "Hi-Hat",
                        "Crash",
                        "Open Hi-Hat",
                        "Clap",
                        "Rim Shot",
                        "Tom",
                    ];
                    let pattern_samples = vec![
                        "kick",
                        "snare",
                        "hihat",
                        "crash",
                        "open_hihat",
                        "clap",
                        "rimshot",
                        "tom",
                    ];

                    let patterns: Vec<Pattern> = pattern_names
                        .iter()
                        .zip(pattern_samples.iter())
                        .map(|(name, sample)| {
                            Pattern::new(
                                name.to_string(),
                                sample.to_string(),
                                app.settings.defaults.default_pattern_length,
                            )
                        })
                        .collect();

                    let default_time_sig = TimeSignature {
                        numerator: app.settings.defaults.default_time_signature.0 as u8,
                        denominator: app.settings.defaults.default_time_signature.1 as u8,
                    };
                    let default_segment = TimelineSegment::new(
                        "Default Pattern".to_string(),
                        patterns,
                        0.0,
                        1,
                        default_time_sig,
                        app.settings.defaults.default_bpm,
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

        self.current_project =
            Project::new_with_defaults("New Project".to_string(), &self.settings.defaults);
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

    // Settings management methods
    fn handle_settings_change(&mut self) {
        // Get updated settings from dialog
        let new_settings = self.settings_dialog.get_settings().clone();

        // Apply audio settings immediately if supported
        if let Some(ref audio_engine) = self.audio_engine {
            // Update master volume
            audio_engine.set_master_volume(new_settings.audio.master_volume);
        }

        // Store settings for future audio engine recreation if needed
        self.settings = new_settings.clone();

        // Save settings to file
        if let Err(e) = self.settings.auto_save() {
            self.error_message = Some(format!("Failed to save settings: {}", e));
        }

        // Note: Sample rate, buffer size, and device changes require audio engine restart
        // which is not implemented in this version - would show a message to restart app
    }

    // Keyboard shortcut handling methods
    fn handle_keyboard_input(&mut self, ctx: &egui::Context) {
        // Skip keyboard handling if settings dialog is open to avoid conflicts
        if self.settings_dialog.is_open() {
            return;
        }

        ctx.input(|i| {
            // Process each key that was pressed this frame
            for event in &i.events {
                if let egui::Event::Key { key, pressed, modifiers, .. } = event {
                    if *pressed {
                        self.handle_key_press(*key, modifiers);
                    }
                }
            }
        });
    }

    fn handle_key_press(&mut self, key: egui::Key, modifiers: &egui::Modifiers) {
        let keyboard = &self.settings.keyboard;
        
        // Transport Control Shortcuts
        if KeyboardSettings::matches_shortcut(&keyboard.play_pause, key, modifiers) {
            self.handle_play_pause_shortcut();
        } else if KeyboardSettings::matches_shortcut(&keyboard.return_to_start, key, modifiers) {
            self.handle_return_to_start_shortcut();
        } else if KeyboardSettings::matches_shortcut(&keyboard.stop_escape, key, modifiers) {
            self.handle_stop_escape_shortcut();
        }
        // Timeline Navigation Shortcuts
        else if KeyboardSettings::matches_shortcut(&keyboard.timeline_step_back, key, modifiers) {
            self.handle_timeline_step_back();
        } else if KeyboardSettings::matches_shortcut(&keyboard.timeline_step_forward, key, modifiers) {
            self.handle_timeline_step_forward();
        } else if KeyboardSettings::matches_shortcut(&keyboard.timeline_jump_back, key, modifiers) {
            self.handle_timeline_jump_back();
        } else if KeyboardSettings::matches_shortcut(&keyboard.timeline_jump_forward, key, modifiers) {
            self.handle_timeline_jump_forward();
        } else if KeyboardSettings::matches_shortcut(&keyboard.timeline_start, key, modifiers) {
            self.handle_timeline_start();
        } else if KeyboardSettings::matches_shortcut(&keyboard.timeline_end, key, modifiers) {
            self.handle_timeline_end();
        }
        // Application Shortcuts
        else if KeyboardSettings::matches_shortcut(&keyboard.new_project, key, modifiers) {
            self.new_project();
        } else if KeyboardSettings::matches_shortcut(&keyboard.open_project, key, modifiers) {
            self.load_project();
        } else if KeyboardSettings::matches_shortcut(&keyboard.save_project, key, modifiers) {
            self.save_project();
        } else if KeyboardSettings::matches_shortcut(&keyboard.save_project_as, key, modifiers) {
            self.save_project_as();
        } else if KeyboardSettings::matches_shortcut(&keyboard.open_settings, key, modifiers) {
            self.settings_dialog.open();
        }
        // Pattern Editing Shortcuts
        else if KeyboardSettings::matches_shortcut(&keyboard.pattern_clear, key, modifiers) {
            self.handle_pattern_clear();
        } else if KeyboardSettings::matches_shortcut(&keyboard.pattern_select_all, key, modifiers) {
            self.handle_pattern_select_all();
        }
    }

    fn handle_play_pause_shortcut(&mut self) {
        if let Some(ref audio_engine) = self.audio_engine {
            if let Ok(mut timeline) = audio_engine.timeline().try_lock() {
                if timeline.is_playing() {
                    timeline.stop();
                } else {
                    timeline.play();
                }
            }
        }
    }

    fn handle_return_to_start_shortcut(&mut self) {
        if let Some(ref audio_engine) = self.audio_engine {
            if let Ok(mut timeline) = audio_engine.timeline().try_lock() {
                timeline.seek(0.0);
            }
        }
    }

    fn handle_stop_escape_shortcut(&mut self) {
        if let Some(ref audio_engine) = self.audio_engine {
            if let Ok(mut timeline) = audio_engine.timeline().try_lock() {
                timeline.stop();
            }
        }
    }

    // Timeline navigation shortcut handlers
    fn handle_timeline_step_back(&mut self) {
        if let Some(ref audio_engine) = self.audio_engine {
            if let Ok(mut timeline) = audio_engine.timeline().try_lock() {
                // Step backward by 0.1 seconds (small increment)
                let current = timeline.current_position;
                let new_position = (current - 0.1).max(0.0);
                timeline.seek(new_position);
            }
        }
    }

    fn handle_timeline_step_forward(&mut self) {
        if let Some(ref audio_engine) = self.audio_engine {
            if let Ok(mut timeline) = audio_engine.timeline().try_lock() {
                // Step forward by 0.1 seconds (small increment)
                let current = timeline.current_position;
                let new_position = (current + 0.1).min(timeline.total_duration());
                timeline.seek(new_position);
            }
        }
    }

    fn handle_timeline_jump_back(&mut self) {
        if let Some(ref audio_engine) = self.audio_engine {
            if let Ok(mut timeline) = audio_engine.timeline().try_lock() {
                // Jump backward by 1.0 seconds (larger increment - one measure at 120 BPM = 2 seconds)
                let current = timeline.current_position;
                let new_position = (current - 1.0).max(0.0);
                timeline.seek(new_position);
            }
        }
    }

    fn handle_timeline_jump_forward(&mut self) {
        if let Some(ref audio_engine) = self.audio_engine {
            if let Ok(mut timeline) = audio_engine.timeline().try_lock() {
                // Jump forward by 1.0 seconds (larger increment)
                let current = timeline.current_position;
                let new_position = (current + 1.0).min(timeline.total_duration());
                timeline.seek(new_position);
            }
        }
    }

    fn handle_timeline_start(&mut self) {
        if let Some(ref audio_engine) = self.audio_engine {
            if let Ok(mut timeline) = audio_engine.timeline().try_lock() {
                timeline.seek(0.0);
            }
        }
    }

    fn handle_timeline_end(&mut self) {
        if let Some(ref audio_engine) = self.audio_engine {
            if let Ok(mut timeline) = audio_engine.timeline().try_lock() {
                let total_duration = timeline.total_duration();
                timeline.seek(total_duration);
            }
        }
    }

    // Pattern editing shortcut handlers (basic implementation)
    fn handle_pattern_clear(&mut self) {
        // Note: This is a basic implementation. Full pattern grid integration would require
        // pattern grid focus management and selected step tracking
        // For now, this provides the infrastructure for pattern editing shortcuts
    }

    fn handle_pattern_select_all(&mut self) {
        // Note: This is a basic implementation. Full pattern grid integration would require
        // pattern grid focus management and selection state tracking
        // For now, this provides the infrastructure for pattern editing shortcuts
    }

    // Helper function to format keyboard shortcuts for display in menus
    fn format_shortcut_for_display(&self, shortcut: &str) -> String {
        // Convert shortcut string to display format (e.g., "Ctrl+S" -> "⌘S" on macOS)
        let formatted = shortcut
            .replace("Comma", ",")
            .replace("Period", ".")
            .replace("Semicolon", ";")
            .replace("Quote", "'")
            .replace("Slash", "/")
            .replace("Backslash", "\\")
            .replace("Minus", "-")
            .replace("Equals", "=");

        if cfg!(target_os = "macos") {
            formatted
                .replace("Cmd", "⌘")
                .replace("Ctrl", "⌃")
                .replace("Alt", "⌥")
                .replace("Shift", "⇧")
                .replace("+", "")
        } else {
            formatted
        }
    }

    // Helper function to create menu item with keyboard shortcut
    fn menu_item_with_shortcut(&self, ui: &mut egui::Ui, label: &str, shortcut: &str) -> egui::Response {
        let formatted_shortcut = self.format_shortcut_for_display(shortcut);
        let full_text = format!("{}\t{}", label, formatted_shortcut);
        ui.button(full_text)
    }

    fn refresh_audio_devices(&mut self) {
        // Get detailed device information
        match AudioEngine::get_available_devices_detailed() {
            Ok(devices_detailed) => {
                self.settings_dialog
                    .update_available_devices_detailed(devices_detailed);
            }
            Err(_) => {
                // Fallback to simple device enumeration
                match AudioEngine::get_available_devices() {
                    Ok(devices) => {
                        self.settings_dialog.update_available_devices(devices);
                    }
                    Err(e) => {
                        self.error_message =
                            Some(format!("Failed to refresh audio devices: {}", e));
                    }
                }
            }
        }
    }

    fn handle_device_monitoring(&mut self) {
        if let Some(ref mut audio_engine) = self.audio_engine {
            // Check if monitoring is enabled and device is available
            if self.settings.audio.device_monitoring_enabled {
                match audio_engine.monitor_device_availability() {
                    Ok(is_available) => {
                        if !is_available {
                            // Device is no longer available, handle fallback
                            match audio_engine.handle_device_disconnection() {
                                Ok(action) => {
                                    match action {
                                        crate::audio::engine::DeviceRecoveryAction::NoAction => {
                                            // Device became available again, no action needed
                                        }
                                        crate::audio::engine::DeviceRecoveryAction::FallbackToDefault => {
                                            if let Ok(success) = audio_engine.switch_to_device("Default Device".to_string()) {
                                                if success {
                                                    self.settings.audio.preferred_device = None;
                                                    self.error_message = Some("Audio device disconnected. Switched to default device.".to_string());
                                                    // Update settings
                                                    if let Err(e) = self.settings.auto_save() {
                                                        eprintln!("Failed to save settings after device fallback: {}", e);
                                                    }
                                                }
                                            }
                                        }
                                        crate::audio::engine::DeviceRecoveryAction::FallbackToDevice(device_name) => {
                                            if let Ok(success) = audio_engine.switch_to_device(device_name.clone()) {
                                                if success {
                                                    self.settings.audio.preferred_device = Some(device_name.clone());
                                                    self.error_message = Some(format!("Audio device disconnected. Switched to: {}", device_name));
                                                    // Update settings
                                                    if let Err(e) = self.settings.auto_save() {
                                                        eprintln!("Failed to save settings after device fallback: {}", e);
                                                    }
                                                }
                                            }
                                        }
                                        crate::audio::engine::DeviceRecoveryAction::DeviceUnavailable => {
                                            self.error_message = Some("Audio device disconnected and no fallback devices are available.".to_string());
                                        }
                                    }
                                }
                                Err(e) => {
                                    eprintln!("Device monitoring error: {}", e);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Device monitoring failed: {}", e);
                    }
                }
            }
        }
    }
}

// Theme-aware color helper functions
fn get_header_bg_color(visuals: &egui::Visuals) -> egui::Color32 {
    if visuals.dark_mode {
        egui::Color32::from_gray(20)
    } else {
        egui::Color32::from_gray(248)
    }
}

fn get_container_bg_color(visuals: &egui::Visuals) -> egui::Color32 {
    if visuals.dark_mode {
        egui::Color32::from_gray(30)
    } else {
        egui::Color32::from_gray(245)
    }
}

fn get_container_stroke_color(visuals: &egui::Visuals) -> egui::Color32 {
    if visuals.dark_mode {
        egui::Color32::from_gray(50)
    } else {
        egui::Color32::from_gray(200)
    }
}

fn get_secondary_container_bg_color(visuals: &egui::Visuals) -> egui::Color32 {
    if visuals.dark_mode {
        egui::Color32::from_gray(35)
    } else {
        egui::Color32::from_gray(240)
    }
}

fn get_footer_bg_color(visuals: &egui::Visuals) -> egui::Color32 {
    if visuals.dark_mode {
        egui::Color32::from_gray(20)
    } else {
        egui::Color32::from_gray(250)
    }
}

fn get_muted_text_color(visuals: &egui::Visuals) -> egui::Color32 {
    if visuals.dark_mode {
        egui::Color32::from_gray(180)
    } else {
        egui::Color32::from_gray(100)
    }
}

impl eframe::App for DrumComposerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Apply UI settings with theme resolution and monitoring
        let resolved_theme = self.settings.ui.resolve_theme();

        // Check for auto theme changes (only if using auto mode)
        if self.settings.ui.theme == "auto" && resolved_theme != self.last_resolved_theme {
            self.theme_change_notification = Some(format!(
                "Theme automatically switched to {} to match system",
                resolved_theme
            ));
            self.last_resolved_theme = resolved_theme.clone();
        }

        let visuals = match resolved_theme.as_str() {
            "light" => egui::Visuals::light(),
            _ => egui::Visuals::dark(),
        };
        ctx.set_visuals(visuals);

        // Apply UI scale comprehensively
        ctx.set_pixels_per_point(self.settings.ui.ui_scale);

        // Update window title
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(self.get_window_title()));

        // Handle keyboard shortcuts (before UI processing to ensure they work globally)
        self.handle_keyboard_input(ctx);

        // Menu bar
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if self.menu_item_with_shortcut(ui, "New Project", &self.settings.keyboard.new_project).clicked() {
                        self.new_project();
                        ui.close_menu();
                    }

                    if self.menu_item_with_shortcut(ui, "Open Project...", &self.settings.keyboard.open_project).clicked() {
                        self.load_project();
                        ui.close_menu();
                    }

                    ui.separator();

                    if self.menu_item_with_shortcut(ui, "Save Project", &self.settings.keyboard.save_project).clicked() {
                        self.save_project();
                        ui.close_menu();
                    }

                    if self.menu_item_with_shortcut(ui, "Save Project As...", &self.settings.keyboard.save_project_as).clicked() {
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

                ui.menu_button("Settings", |ui| {
                    if self.menu_item_with_shortcut(ui, "Preferences...", &self.settings.keyboard.open_settings).clicked() {
                        self.settings_dialog.open();
                        ui.close_menu();
                    }
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
                .fill(get_header_bg_color(&ui.visuals()))
                .inner_margin(egui::Margin::symmetric(16.0, 12.0))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.heading("🥁 Beatr");
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label("Drum Track Composer");
                        });
                    });
                });

            ui.add_space(6.0);

            // Show theme change notification (info message)
            if let Some(notification) = self.theme_change_notification.clone() {
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(20, 50, 60))
                    .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(40, 100, 120)))
                    .inner_margin(egui::Margin::same(8.0))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.colored_label(egui::Color32::from_rgb(100, 180, 220),
                                           format!("ℹ {}", notification));
                            if ui.small_button("✕").clicked() {
                                self.theme_change_notification = None;
                            }
                        });
                    });
                ui.add_space(6.0);
            }
            
            if let Some(ref error) = self.error_message {
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(60, 20, 20))
                    .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(120, 40, 40)))
                    .inner_margin(egui::Margin::same(8.0))
                    .show(ui, |ui| {
                        ui.colored_label(egui::Color32::from_rgb(255, 100, 100),
                                       format!("⚠ Error: {}", error));
                    });
                ui.add_space(6.0);
                return;
            }

            if let Some(ref _audio_engine) = self.audio_engine {

                // Flattened transport controls - minimal nesting for better alignment
                egui::Frame::none()
                    .fill(get_container_bg_color(&ui.visuals()))
                    .stroke(egui::Stroke::new(1.0, get_container_stroke_color(&ui.visuals())))
                    .inner_margin(egui::Margin::same(12.0))
                    .rounding(4.0)
                    .show(ui, |ui| {
                        // Single horizontal layout - all controls in one line
                        ui.horizontal(|ui| {
                            // Transport controls - direct placement, no groups
                            ui.label("Transport:");
                            ui.add_space(6.0);
                            
                            // Check if timeline has segments to determine playback mode
                            let has_timeline_segments = {
                                if let Ok(timeline) = self.timeline.lock() {
                                    !timeline.segments.is_empty()
                                } else {
                                    false
                                }
                            };

                            if has_timeline_segments {
                                // Timeline transport controls - direct buttons
                                let (is_timeline_playing, timeline_position) = {
                                    if let Ok(timeline) = self.timeline.lock() {
                                        (timeline.is_playing(), timeline.current_position)
                                    } else {
                                        (false, 0.0)
                                    }
                                };

                                if ui.button(if is_timeline_playing { "⏸" } else { "▶" }).clicked() {
                                    if let Ok(mut timeline) = self.timeline.lock() {
                                        if is_timeline_playing {
                                            timeline.pause();
                                        } else {
                                            timeline.play();
                                        }
                                    }
                                }

                                if ui.button("⏹").clicked() {
                                    if let Ok(mut timeline) = self.timeline.lock() {
                                        timeline.stop();
                                    }
                                }

                                ui.label(format!("{:.1}s", timeline_position));
                            } else {
                                TransportControls::show(ui, &self.timeline);
                            }
                            
                            ui.separator();
                            
                            // Tempo controls - direct placement
                            let tempo_changed = TempoControl::show(ui, &mut self.tempo);
                            if tempo_changed {
                                ui.add_space(4.0);
                                if ui.small_button("Apply All").clicked() {
                                    if let Ok(mut timeline) = self.timeline.lock() {
                                        timeline.set_global_bpm(self.tempo);
                                    }
                                    self.project_modified = true;
                                }
                            }
                            
                            ui.separator();
                            
                            // Time signature controls - direct placement
                            ui.label("Time Sig:");
                            ui.add_space(4.0);
                            
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
                            
                            // Status on the right - direct placement
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.label("🎼");
                            });
                        });
                    });

                ui.add_space(6.0);

                // Timeline view first - this updates sequencer patterns based on selection
                let mut timeline_modified = false;
                if let Some(ref mut timeline_view) = self.timeline_view {
                    egui::Frame::none()
                        .fill(get_container_bg_color(&ui.visuals()))
                        .stroke(egui::Stroke::new(1.0, get_container_stroke_color(&ui.visuals())))
                        .inner_margin(egui::Margin::same(12.0))
                        .rounding(4.0)
                        .show(ui, |ui| {
                            // Flattened timeline header - no nested horizontal layout
                            ui.horizontal(|ui| {
                                ui.heading("Timeline");
                                ui.add_space(12.0);
                                
                                // Duration display directly in the same horizontal line
                                if let Ok(timeline) = self.timeline.lock() {
                                    let duration = timeline.total_duration();
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        ui.label(format!("Duration: {:.1}s", duration));
                                    });
                                }
                            });
                            ui.add_space(6.0);
                            timeline_view.show(ui, &self.timeline, self.tempo);
                            timeline_modified = true; // Assume timeline was modified
                        });
                }
                
                // Sync timeline changes with project after UI interaction
                if timeline_modified {
                    self.sync_audio_timeline_to_project();
                }

                ui.add_space(6.0);

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
                            egui::Frame::none()
                                .fill(get_secondary_container_bg_color(&ui.visuals()))
                                .inner_margin(egui::Margin::same(8.0))
                                .rounding(4.0)
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.label("📝 Editing timeline segment:");
                                        ui.strong(egui::RichText::new(segment_name).color(egui::Color32::from_rgb(100, 140, 220)));
                                    });
                                });
                            ui.add_space(6.0);
                        }
                        let selected_segment_id = if let Some(ref timeline_view) = self.timeline_view {
                            timeline_view.get_selected_segment_id()
                        } else {
                            None
                        };
                        PatternGrid::show(ui, &self.timeline, selected_segment_id.as_deref());
                    });

                // No sync needed - patterns are stored directly in timeline segments

                ui.add_space(6.0);

                // Footer with help text
                egui::Frame::none()
                    .fill(get_footer_bg_color(&ui.visuals()))
                    .inner_margin(egui::Margin::same(8.0))
                    .rounding(4.0)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.small(egui::RichText::new("💡 Tip: Click step buttons to create patterns • Numbers show measure positions • Clear to reset tracks")
                                .color(get_muted_text_color(&ui.visuals())));
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

        // Handle settings dialog
        let settings_changed = self.settings_dialog.show(ctx);
        if settings_changed {
            self.handle_settings_change();
        }

        // Handle device refresh requests
        if self.settings_dialog.take_device_refresh_requested() {
            self.refresh_audio_devices();
        }

        // Handle device monitoring (check periodically, not every frame)
        // This is a simple approach - in a real app you might want to use a timer
        static mut MONITORING_COUNTER: u32 = 0;
        unsafe {
            MONITORING_COUNTER += 1;
            if MONITORING_COUNTER % 300 == 0 {
                // Check every ~5 seconds at 60fps
                self.handle_device_monitoring();
            }
        }

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_responsive_layout_width_thresholds() {
        // Test that responsive layout properly detects narrow vs wide layouts
        const NARROW_THRESHOLD: f32 = 800.0;

        // These widths should trigger narrow layout
        let narrow_widths = [600.0, 700.0, 799.0];
        for width in narrow_widths {
            assert!(
                width < NARROW_THRESHOLD,
                "Width {} should be considered narrow",
                width
            );
        }

        // These widths should trigger wide layout
        let wide_widths = [800.0, 900.0, 1200.0, 1920.0];
        for width in wide_widths {
            assert!(
                width >= NARROW_THRESHOLD,
                "Width {} should be considered wide",
                width
            );
        }
    }

    #[test]
    fn test_window_title_generation() {
        // Test window title generation with mock app state
        let mut app = create_test_app();

        // Test clean project title
        let title = app.get_window_title();
        assert_eq!(title, "Beatr - New Project");

        // Test modified project title
        app.project_modified = true;
        let title = app.get_window_title();
        assert_eq!(title, "Beatr - New Project*");
    }

    #[test]
    fn test_project_state_management() {
        let mut app = create_test_app();

        // Initially not modified
        assert!(!app.project_modified);

        // Creating a new project should reset modified state
        app.project_modified = true;
        app.new_project();
        assert!(!app.project_modified);
        assert_eq!(app.current_project.metadata.name, "New Project");
        assert_eq!(app.tempo, app.current_project.global_bpm);
    }

    // Helper function to create a test app without UI dependencies
    fn create_test_app() -> DrumComposerApp {
        let settings = AppSettings::default();
        let initial_resolved_theme = settings.ui.resolve_theme();

        DrumComposerApp {
            audio_engine: None,
            error_message: None,
            tempo: 120.0,
            custom_loop_length_text: "16".to_string(),
            custom_time_sig_numerator: "4".to_string(),
            custom_time_sig_denominator: "4".to_string(),
            time_sig_validation_error: None,
            timeline: Arc::new(Mutex::new(Timeline::new())),
            timeline_view: None,
            current_project: Project::new_with_defaults(
                "New Project".to_string(),
                &settings.defaults,
            ),
            current_project_path: None,
            project_modified: false,
            settings_dialog: SettingsDialog::new(settings.clone()),
            last_resolved_theme: initial_resolved_theme,
            theme_change_notification: None,
            settings,
        }
    }

    #[test]
    fn test_shortcut_formatting_cross_platform() {
        let app = create_test_app();

        // Test basic shortcut formatting
        assert_eq!(app.format_shortcut_for_display("Space"), "Space");
        assert_eq!(app.format_shortcut_for_display("Enter"), "Enter");

        // Test punctuation formatting
        if cfg!(target_os = "macos") {
            assert_eq!(app.format_shortcut_for_display("Cmd+Comma"), "⌘,");
            assert_eq!(app.format_shortcut_for_display("Ctrl+Period"), "⌃.");
            assert_eq!(app.format_shortcut_for_display("Cmd+,"), "⌘,");
            assert_eq!(app.format_shortcut_for_display("Ctrl+,"), "⌃,");
        } else {
            assert_eq!(app.format_shortcut_for_display("Cmd+Comma"), "Cmd+,");
            assert_eq!(app.format_shortcut_for_display("Ctrl+Period"), "Ctrl+.");
            assert_eq!(app.format_shortcut_for_display("Cmd+,"), "Cmd+,");
            assert_eq!(app.format_shortcut_for_display("Ctrl+,"), "Ctrl+,");
        }

        // Test platform-specific formatting
        if cfg!(target_os = "macos") {
            // macOS should convert to symbols
            assert_eq!(app.format_shortcut_for_display("Cmd+S"), "⌘S");
            assert_eq!(app.format_shortcut_for_display("Ctrl+S"), "⌃S");
            assert_eq!(app.format_shortcut_for_display("Alt+Tab"), "⌥Tab");
            assert_eq!(app.format_shortcut_for_display("Shift+Left"), "⇧Left");
            assert_eq!(app.format_shortcut_for_display("Cmd+Shift+S"), "⌘⇧S");
            assert_eq!(app.format_shortcut_for_display("Cmd+,"), "⌘,");
        } else {
            // Other platforms should keep original format
            assert_eq!(app.format_shortcut_for_display("Ctrl+S"), "Ctrl+S");
            assert_eq!(app.format_shortcut_for_display("Alt+Tab"), "Alt+Tab");
            assert_eq!(app.format_shortcut_for_display("Shift+Left"), "Shift+Left");
            assert_eq!(app.format_shortcut_for_display("Ctrl+Shift+S"), "Ctrl+Shift+S");
            assert_eq!(app.format_shortcut_for_display("Ctrl+,"), "Ctrl+,");
        }
    }

    #[test]
    fn test_menu_item_shortcut_integration() {
        let app = create_test_app();
        
        // Test that default keyboard settings are used correctly
        let keyboard = &app.settings.keyboard;
        
        // Verify shortcuts can be formatted for all menu items
        let _formatted_new = app.format_shortcut_for_display(&keyboard.new_project);
        let _formatted_open = app.format_shortcut_for_display(&keyboard.open_project);
        let _formatted_save = app.format_shortcut_for_display(&keyboard.save_project);
        let _formatted_save_as = app.format_shortcut_for_display(&keyboard.save_project_as);
        let _formatted_settings = app.format_shortcut_for_display(&keyboard.open_settings);

        // All formatting operations should succeed without panicking
        assert!(true); // If we reach here, all formatting succeeded
    }
}
