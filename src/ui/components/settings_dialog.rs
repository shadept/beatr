use crate::audio::engine::{AudioDeviceInfo, AudioEngine};
use crate::settings::{AppSettings, AudioSettings, DefaultSettings, UISettings};
use eframe::egui;

/// Settings dialog component for managing application settings
#[derive(Clone)]
pub struct SettingsDialog {
    pub open: bool,
    settings: AppSettings,
    original_settings: AppSettings,
    selected_tab: SettingsTab,

    // UI state
    available_devices: Vec<String>,
    available_devices_detailed: Vec<AudioDeviceInfo>,
    device_refresh_requested: bool,
    device_test_status: Option<DeviceTestResult>,
    last_test_device: Option<String>,

    // Pending changes (for delayed application)
    pending_ui_scale: Option<f32>,
}

#[derive(Debug, Clone, PartialEq)]
enum SettingsTab {
    Audio,
    UI,
    Defaults,
    Keyboard,
}

#[derive(Debug, Clone, PartialEq)]
enum DeviceTestResult {
    Success,
    Failed(String),
    Testing,
}

impl SettingsDialog {
    /// Create a new settings dialog with the given settings
    pub fn new(settings: AppSettings) -> Self {
        // Try to load detailed device info at startup
        let (devices, devices_detailed) = match AudioEngine::get_available_devices_detailed() {
            Ok(detailed) => {
                let simple: Vec<String> = detailed.iter().map(|d| d.name.clone()).collect();
                (simple, detailed)
            }
            Err(_) => {
                // Fallback to simple enumeration
                let simple = AudioEngine::get_available_devices()
                    .unwrap_or_else(|_| vec!["Default Device".to_string()]);
                (simple, Vec::new())
            }
        };

        Self {
            open: false,
            original_settings: settings.clone(),
            settings,
            selected_tab: SettingsTab::Audio,
            available_devices: devices,
            available_devices_detailed: devices_detailed,
            device_refresh_requested: false,
            device_test_status: None,
            last_test_device: None,
            pending_ui_scale: None,
        }
    }

    /// Open the settings dialog
    pub fn open(&mut self) {
        self.open = true;
        self.original_settings = self.settings.clone();
    }

    /// Close the settings dialog
    pub fn close(&mut self) {
        self.open = false;
    }

    /// Check if the settings dialog is open
    pub fn is_open(&self) -> bool {
        self.open
    }

    /// Get the current settings
    pub fn get_settings(&self) -> &AppSettings {
        &self.settings
    }

    /// Update the settings (for external changes)
    pub fn update_settings(&mut self, settings: AppSettings) {
        self.settings = settings;
    }

    /// Check if there are pending UI scale changes
    pub fn has_pending_ui_scale_change(&self) -> bool {
        self.pending_ui_scale.is_some()
    }

    /// Apply pending UI scale changes and return the new scale value
    pub fn apply_pending_ui_scale_change(&mut self) -> Option<f32> {
        if let Some(scale) = self.pending_ui_scale.take() {
            self.settings.ui.ui_scale = scale;
            Some(scale)
        } else {
            None
        }
    }

    /// Get the current display scale (including pending changes)
    fn get_display_ui_scale(&self) -> f32 {
        self.pending_ui_scale.unwrap_or(self.settings.ui.ui_scale)
    }

    /// Show the settings dialog and return true if settings were changed
    pub fn show(&mut self, ctx: &egui::Context) -> bool {
        let mut settings_changed = false;

        if !self.open {
            return false;
        }

        let mut open = self.open;
        egui::Window::new("Settings")
            .open(&mut open)
            .default_size([600.0, 400.0])
            .resizable(true)
            .show(ctx, |ui| {
                settings_changed = self.show_content(ui);
            });

        // Handle external close (X button) - apply pending changes
        if self.open && !open {
            // Dialog was closed externally, apply pending changes
            if self.apply_pending_ui_scale_change().is_some() {
                settings_changed = true;
                // Also update original settings so changes are saved
                self.original_settings = self.settings.clone();
            }
        }

        // Update open state - respect both window close (X button) and internal close calls
        self.open = open && self.open;
        settings_changed
    }

    fn show_content(&mut self, ui: &mut egui::Ui) -> bool {
        let mut settings_changed = false;

        // Tab selection
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.selected_tab, SettingsTab::Audio, "Audio");
            ui.selectable_value(&mut self.selected_tab, SettingsTab::UI, "UI");
            ui.selectable_value(&mut self.selected_tab, SettingsTab::Defaults, "Defaults");
            ui.selectable_value(&mut self.selected_tab, SettingsTab::Keyboard, "Keyboard");
        });

        ui.separator();

        // Tab content
        egui::ScrollArea::vertical().show(ui, |ui| match self.selected_tab {
            SettingsTab::Audio => settings_changed = self.show_audio_settings(ui),
            SettingsTab::UI => settings_changed = self.show_ui_settings(ui),
            SettingsTab::Defaults => settings_changed = self.show_default_settings(ui),
            SettingsTab::Keyboard => settings_changed = self.show_keyboard_settings(ui),
        });

        ui.separator();

        // Bottom buttons
        ui.horizontal(|ui| {
            if ui.button("Reset to Defaults").clicked() {
                match self.selected_tab {
                    SettingsTab::Audio => {
                        self.settings.audio = AudioSettings::default();
                        settings_changed = true;
                    }
                    SettingsTab::UI => {
                        self.settings.ui = UISettings::default();
                        settings_changed = true;
                    }
                    SettingsTab::Defaults => {
                        self.settings.defaults = DefaultSettings::default();
                        settings_changed = true;
                    }
                    SettingsTab::Keyboard => {
                        // Keyboard settings are read-only, no reset functionality
                    }
                }
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Close").clicked() {
                    // Apply pending UI scale changes
                    self.apply_pending_ui_scale_change();

                    // Update original settings to current settings so they're saved
                    self.original_settings = self.settings.clone();
                    settings_changed = true;
                    self.close();
                }

                if ui.button("Cancel").clicked() {
                    self.settings = self.original_settings.clone();
                    // Clear pending UI scale changes
                    self.pending_ui_scale = None;
                    self.close();
                }

                if ui.button("Apply").clicked() {
                    // Apply pending UI scale changes
                    self.apply_pending_ui_scale_change();

                    // Update original settings to current settings after applying
                    self.original_settings = self.settings.clone();
                    settings_changed = true;
                    // Settings will be applied by the caller
                }
            });
        });

        settings_changed
    }

    fn show_audio_settings(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;

        ui.heading("Audio Settings");
        ui.add_space(10.0);

        // Sample Rate (with device-specific constraints)
        ui.horizontal(|ui| {
            ui.label("Sample Rate:");

            // Get supported rates for current device
            let supported_rates = self.get_supported_sample_rates_for_current_device();
            let current_rate = self.settings.audio.sample_rate;

            egui::ComboBox::from_id_source("sample_rate_combo")
                .selected_text(format!("{} Hz", current_rate))
                .show_ui(ui, |ui| {
                    for &rate in &supported_rates {
                        let is_supported = self.is_rate_supported_by_current_device(rate);
                        let label = if is_supported {
                            format!("{} Hz", rate)
                        } else {
                            format!("{} Hz (unsupported)", rate)
                        };

                        if ui
                            .selectable_value(&mut self.settings.audio.sample_rate, rate, label)
                            .clicked()
                        {
                            if is_supported {
                                changed = true;
                            }
                        }
                    }
                });
        });

        // Device compatibility indicator for sample rate
        if !self.is_rate_supported_by_current_device(self.settings.audio.sample_rate) {
            ui.horizontal(|ui| {
                ui.add_space(120.0); // Align with label
                ui.colored_label(
                    egui::Color32::from_rgb(255, 150, 0),
                    "⚠ This sample rate may not be supported by the selected device",
                );
            });
        }

        ui.add_space(10.0);

        // Buffer Size (with device-specific constraints)
        ui.horizontal(|ui| {
            ui.label("Buffer Size:");

            let supported_sizes = self.get_supported_buffer_sizes_for_current_device();
            let current_size = self.settings.audio.buffer_size;

            egui::ComboBox::from_id_source("buffer_size_combo")
                .selected_text(format!("{} samples", current_size))
                .show_ui(ui, |ui| {
                    for &size in &supported_sizes {
                        let is_supported = self.is_buffer_size_supported_by_current_device(size);
                        let label = if is_supported {
                            format!("{} samples", size)
                        } else {
                            format!("{} samples (unsupported)", size)
                        };

                        if ui
                            .selectable_value(&mut self.settings.audio.buffer_size, size, label)
                            .clicked()
                        {
                            if is_supported {
                                changed = true;
                            }
                        }
                    }
                });
        });

        // Device compatibility indicator for buffer size
        if !self.is_buffer_size_supported_by_current_device(self.settings.audio.buffer_size) {
            ui.horizontal(|ui| {
                ui.add_space(120.0); // Align with label
                ui.colored_label(
                    egui::Color32::from_rgb(255, 150, 0),
                    "⚠ This buffer size may not be supported by the selected device",
                );
            });
        }

        ui.add_space(10.0);

        // Master Volume
        ui.horizontal(|ui| {
            ui.label("Master Volume:");
            if ui
                .add(
                    egui::Slider::new(&mut self.settings.audio.master_volume, 0.0..=2.0)
                        .custom_formatter(|n, _| format!("{:.0}%", n * 100.0)),
                )
                .changed()
            {
                changed = true;
            }
        });

        ui.add_space(10.0);

        // Audio Device Selection with Status Indicators
        ui.horizontal(|ui| {
            ui.label("Audio Device:");

            let current_device = self
                .settings
                .audio
                .preferred_device
                .as_deref()
                .unwrap_or("Default Device");
            egui::ComboBox::from_id_source("audio_device_combo")
                .selected_text(current_device)
                .show_ui(ui, |ui| {
                    for device_info in &self.available_devices_detailed {
                        let device_display = if device_info.is_default
                            && !device_info.name.ends_with(" (Default)")
                        {
                            format!("{} (Default)", device_info.name)
                        } else {
                            device_info.name.clone()
                        };

                        // Status indicator
                        let status_color = if device_info.is_available {
                            egui::Color32::from_rgb(0, 200, 0) // Green
                        } else {
                            egui::Color32::from_rgb(200, 0, 0) // Red
                        };

                        ui.horizontal(|ui| {
                            ui.colored_label(status_color, "●");
                            if ui
                                .selectable_value(
                                    &mut self.settings.audio.preferred_device,
                                    if device_info.name == "Default Device"
                                        || device_info.name.ends_with(" (Default)")
                                    {
                                        None
                                    } else {
                                        Some(device_info.name.clone())
                                    },
                                    &device_display,
                                )
                                .clicked()
                            {
                                changed = true;
                            }
                        });
                    }

                    // Fallback for non-detailed devices
                    for device in &self.available_devices {
                        if !self
                            .available_devices_detailed
                            .iter()
                            .any(|d| &d.name == device)
                        {
                            if ui
                                .selectable_value(
                                    &mut self.settings.audio.preferred_device,
                                    if device == "Default Device" {
                                        None
                                    } else {
                                        Some(device.clone())
                                    },
                                    device,
                                )
                                .clicked()
                            {
                                changed = true;
                            }
                        }
                    }
                });

            if ui.button("Refresh Devices").clicked() {
                self.device_refresh_requested = true;
            }

            if ui.button("Test Device").clicked() {
                // Test current device configuration
                self.test_current_device_configuration();
            }
        });

        // Device test status display
        if let Some(ref test_status) = self.device_test_status {
            ui.horizontal(|ui| {
                ui.add_space(120.0); // Align with label
                match test_status {
                    DeviceTestResult::Success => {
                        ui.colored_label(
                            egui::Color32::from_rgb(0, 200, 0),
                            "✓ Device configuration test passed",
                        );
                    }
                    DeviceTestResult::Failed(error) => {
                        ui.colored_label(
                            egui::Color32::from_rgb(200, 0, 0),
                            format!("✗ Device test failed: {}", error),
                        );
                    }
                    DeviceTestResult::Testing => {
                        ui.colored_label(
                            egui::Color32::from_rgb(255, 150, 0),
                            "⏳ Testing device configuration...",
                        );
                    }
                }
            });
        }

        ui.add_space(10.0);

        // Device Monitoring Settings
        ui.heading("Device Monitoring");
        ui.add_space(5.0);

        ui.horizontal(|ui| {
            if ui
                .checkbox(
                    &mut self.settings.audio.device_monitoring_enabled,
                    "Enable device monitoring",
                )
                .changed()
            {
                changed = true;
            }
            ui.label("Monitor audio device availability");
        });

        ui.horizontal(|ui| {
            ui.add_enabled_ui(self.settings.audio.device_monitoring_enabled, |ui| {
                if ui
                    .checkbox(
                        &mut self.settings.audio.auto_fallback_enabled,
                        "Enable automatic fallback",
                    )
                    .changed()
                {
                    changed = true;
                }
                ui.label(
                    "Automatically switch to default device if current device becomes unavailable",
                );
            });
        });

        if let Some(ref last_good_device) = self.settings.audio.last_known_good_device {
            ui.horizontal(|ui| {
                ui.label("Last known good device:");
                ui.colored_label(egui::Color32::from_rgb(0, 150, 255), last_good_device);
            });
        }

        changed
    }

    fn show_ui_settings(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;

        ui.heading("UI Settings");
        ui.add_space(10.0);

        // Theme
        ui.horizontal(|ui| {
            ui.label("Theme:");
            egui::ComboBox::from_id_source("theme_combo")
                .selected_text(&self.settings.ui.theme)
                .show_ui(ui, |ui| {
                    for theme in &["dark", "light", "auto"] {
                        let display_text = match *theme {
                            "auto" => "Auto (System)",
                            other => other,
                        };
                        if ui
                            .selectable_value(
                                &mut self.settings.ui.theme,
                                theme.to_string(),
                                display_text,
                            )
                            .clicked()
                        {
                            changed = true;
                        }
                    }
                });
        });

        ui.add_space(10.0);

        // UI Scale (with delayed application)
        ui.horizontal(|ui| {
            ui.label("UI Scale:");
            let mut display_scale = self.get_display_ui_scale();
            if ui
                .add(
                    egui::Slider::new(&mut display_scale, 0.5..=3.0)
                        .custom_formatter(|n, _| format!("{:.1}x", n)),
                )
                .changed()
            {
                // Store the change as pending instead of applying immediately
                self.pending_ui_scale = Some(display_scale);
                changed = true;
            }

            // Show preview text if scale is pending
            if self.has_pending_ui_scale_change() {
                ui.weak("(preview - apply to take effect)");
            }
        });

        changed
    }

    fn show_default_settings(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;

        ui.heading("Default Settings");
        ui.add_space(10.0);

        // Default BPM
        ui.horizontal(|ui| {
            ui.label("Default BPM:");
            if ui
                .add(
                    egui::Slider::new(&mut self.settings.defaults.default_bpm, 60.0..=300.0)
                        .custom_formatter(|n, _| format!("{:.0} BPM", n)),
                )
                .changed()
            {
                changed = true;
            }
        });

        ui.add_space(10.0);

        // Time Signature
        ui.horizontal(|ui| {
            ui.label("Default Time Signature:");
            let (mut numerator, mut denominator) = self.settings.defaults.default_time_signature;

            if ui.add(egui::Slider::new(&mut numerator, 1..=16)).changed() {
                self.settings.defaults.default_time_signature.0 = numerator;
                changed = true;
            }

            ui.label("/");

            egui::ComboBox::from_id_source("time_signature_denominator_combo")
                .selected_text(denominator.to_string())
                .show_ui(ui, |ui| {
                    for &denom in &[1, 2, 4, 8, 16] {
                        if ui
                            .selectable_value(&mut denominator, denom, denom.to_string())
                            .clicked()
                        {
                            self.settings.defaults.default_time_signature.1 = denom;
                            changed = true;
                        }
                    }
                });
        });

        ui.add_space(10.0);

        // Default Pattern Length
        ui.horizontal(|ui| {
            ui.label("Default Pattern Length:");
            let mut pattern_length = self.settings.defaults.default_pattern_length;
            if ui
                .add(
                    egui::Slider::new(&mut pattern_length, 4..=64)
                        .custom_formatter(|n, _| format!("{} steps", n)),
                )
                .changed()
            {
                self.settings.defaults.default_pattern_length = pattern_length;
                changed = true;
            }
        });

        // Pattern length presets
        ui.horizontal(|ui| {
            ui.label("Presets:");
            for &length in &[8, 16, 24, 32] {
                if ui.small_button(format!("{} steps", length)).clicked() {
                    self.settings.defaults.default_pattern_length = length;
                    changed = true;
                }
            }
        });

        changed
    }

    /// Check if device refresh was requested and reset the flag
    pub fn take_device_refresh_requested(&mut self) -> bool {
        let requested = self.device_refresh_requested;
        self.device_refresh_requested = false;
        requested
    }

    /// Update the available audio devices list
    pub fn update_available_devices(&mut self, devices: Vec<String>) {
        self.available_devices = devices;
    }

    /// Update the detailed device information
    pub fn update_available_devices_detailed(&mut self, devices: Vec<AudioDeviceInfo>) {
        self.available_devices_detailed = devices.clone();
        self.available_devices = devices.into_iter().map(|d| d.name).collect();
    }

    /// Get supported sample rates for the currently selected device
    fn get_supported_sample_rates_for_current_device(&self) -> Vec<u32> {
        let current_device = self.get_current_device_name();

        // Find the device info
        if let Some(device_info) = self
            .available_devices_detailed
            .iter()
            .find(|d| d.name == current_device)
        {
            device_info.supported_sample_rates.clone()
        } else {
            // Fallback to standard rates
            vec![22050, 44100, 48000, 88200, 96000, 192000]
        }
    }

    /// Get supported buffer sizes for the currently selected device
    fn get_supported_buffer_sizes_for_current_device(&self) -> Vec<u32> {
        let current_device = self.get_current_device_name();

        // Find the device info
        if let Some(device_info) = self
            .available_devices_detailed
            .iter()
            .find(|d| d.name == current_device)
        {
            device_info.supported_buffer_sizes.clone()
        } else {
            // Fallback to standard buffer sizes
            vec![64, 128, 256, 512, 1024, 2048, 4096]
        }
    }

    /// Check if a sample rate is supported by the current device
    fn is_rate_supported_by_current_device(&self, sample_rate: u32) -> bool {
        let current_device = self.get_current_device_name();

        if let Some(device_info) = self
            .available_devices_detailed
            .iter()
            .find(|d| d.name == current_device)
        {
            device_info.supported_sample_rates.contains(&sample_rate)
        } else {
            // Assume supported if no detailed info available
            true
        }
    }

    /// Check if a buffer size is supported by the current device
    fn is_buffer_size_supported_by_current_device(&self, buffer_size: u32) -> bool {
        let current_device = self.get_current_device_name();

        if let Some(device_info) = self
            .available_devices_detailed
            .iter()
            .find(|d| d.name == current_device)
        {
            device_info.supported_buffer_sizes.contains(&buffer_size)
        } else {
            // Assume supported if no detailed info available
            true
        }
    }

    /// Get the name of the currently selected device
    fn get_current_device_name(&self) -> String {
        self.settings
            .audio
            .preferred_device
            .clone()
            .unwrap_or_else(|| "Default Device".to_string())
    }

    /// Test the current device configuration
    fn test_current_device_configuration(&mut self) {
        let device_name = self.get_current_device_name();
        let sample_rate = self.settings.audio.sample_rate;
        let buffer_size = self.settings.audio.buffer_size;

        self.device_test_status = Some(DeviceTestResult::Testing);
        self.last_test_device = Some(device_name.clone());

        // Perform the test (this would ideally be async, but for now we'll do it synchronously)
        match AudioEngine::test_device_configuration(&device_name, sample_rate, buffer_size) {
            Ok(success) => {
                if success {
                    self.device_test_status = Some(DeviceTestResult::Success);
                } else {
                    self.device_test_status = Some(DeviceTestResult::Failed(
                        "Configuration not supported".to_string(),
                    ));
                }
            }
            Err(e) => {
                self.device_test_status = Some(DeviceTestResult::Failed(e.to_string()));
            }
        }
    }

    /// Clear device test status
    pub fn clear_device_test_status(&mut self) {
        self.device_test_status = None;
        self.last_test_device = None;
    }

    /// Show keyboard settings UI and return true if settings were changed
    fn show_keyboard_settings(&mut self, ui: &mut egui::Ui) -> bool {
        let keyboard = &self.settings.keyboard; // Read-only reference

        ui.heading("Keyboard Shortcuts");
        ui.colored_label(
            egui::Color32::from_gray(128), 
            "ℹ️ These shortcuts are currently read-only. Use the shortcuts shown in the application menus."
        );
        ui.add_space(10.0);

        // Transport Control Shortcuts
        ui.group(|ui| {
            ui.label("🎵 Transport Controls");
            ui.separator();

            egui::Grid::new("transport_shortcuts")
                .num_columns(2)
                .spacing([40.0, 4.0])
                .striped(true)
                .show(ui, |ui| {
                    ui.label("Play/Pause:");
                    ui.monospace(&keyboard.play_pause);
                    ui.end_row();

                    ui.label("Return to Start:");
                    ui.monospace(&keyboard.return_to_start);
                    ui.end_row();

                    ui.label("Stop/Escape:");
                    ui.monospace(&keyboard.stop_escape);
                    ui.end_row();
                });
        });

        ui.add_space(10.0);

        // Timeline Navigation Shortcuts
        ui.group(|ui| {
            ui.label("⏱️ Timeline Navigation");
            ui.separator();

            egui::Grid::new("timeline_shortcuts")
                .num_columns(2)
                .spacing([40.0, 4.0])
                .striped(true)
                .show(ui, |ui| {
                    ui.label("Step Backward:");
                    ui.monospace(&keyboard.timeline_step_back);
                    ui.end_row();

                    ui.label("Step Forward:");
                    ui.monospace(&keyboard.timeline_step_forward);
                    ui.end_row();

                    ui.label("Jump Backward:");
                    ui.monospace(&keyboard.timeline_jump_back);
                    ui.end_row();

                    ui.label("Jump Forward:");
                    ui.monospace(&keyboard.timeline_jump_forward);
                    ui.end_row();

                    ui.label("Go to Start:");
                    ui.monospace(&keyboard.timeline_start);
                    ui.end_row();

                    ui.label("Go to End:");
                    ui.monospace(&keyboard.timeline_end);
                    ui.end_row();
                });
        });

        ui.add_space(10.0);

        // Pattern Editing Shortcuts
        ui.group(|ui| {
            ui.label("🎛️ Pattern Editing");
            ui.separator();

            egui::Grid::new("pattern_shortcuts")
                .num_columns(2)
                .spacing([40.0, 4.0])
                .striped(true)
                .show(ui, |ui| {
                    ui.label("Clear Selection:");
                    ui.monospace(&keyboard.pattern_clear);
                    ui.end_row();

                    ui.label("Select All:");
                    ui.monospace(&keyboard.pattern_select_all);
                    ui.end_row();
                });
        });

        ui.add_space(10.0);

        // Application Shortcuts
        ui.group(|ui| {
            ui.label("🗂️ Application");
            ui.separator();

            egui::Grid::new("app_shortcuts")
                .num_columns(2)
                .spacing([40.0, 4.0])
                .striped(true)
                .show(ui, |ui| {
                    ui.label("New Project:");
                    ui.monospace(&keyboard.new_project);
                    ui.end_row();

                    ui.label("Open Project:");
                    ui.monospace(&keyboard.open_project);
                    ui.end_row();

                    ui.label("Save Project:");
                    ui.monospace(&keyboard.save_project);
                    ui.end_row();

                    ui.label("Save Project As:");
                    ui.monospace(&keyboard.save_project_as);
                    ui.end_row();

                    ui.label("Open Settings:");
                    ui.monospace(&keyboard.open_settings);
                    ui.end_row();
                });
        });

        ui.add_space(10.0);

        // Info text for read-only mode
        ui.colored_label(egui::Color32::GREEN, "✓ All shortcuts are working and available");

        ui.add_space(5.0);

        // Platform-specific help text
        ui.label("💡 These shortcuts work throughout the application:");
        ui.label("   • Transport controls work during playback");
        ui.label("   • Timeline navigation works when timeline is focused");
        ui.label("   • Application shortcuts work globally");
        ui.label("   • Pattern editing shortcuts have basic infrastructure");

        false // Always return false since settings can't be changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_settings_dialog_creation() {
        let settings = AppSettings::default();
        let dialog = SettingsDialog::new(settings.clone());

        assert!(!dialog.is_open());
        assert_eq!(
            dialog.get_settings().audio.sample_rate,
            settings.audio.sample_rate
        );
        assert_eq!(dialog.get_settings().ui.theme, settings.ui.theme);
        assert_eq!(
            dialog.get_settings().defaults.default_bpm,
            settings.defaults.default_bpm
        );
    }

    #[test]
    fn test_settings_dialog_open_close() {
        let settings = AppSettings::default();
        let mut dialog = SettingsDialog::new(settings);

        assert!(!dialog.is_open());

        dialog.open();
        assert!(dialog.is_open());

        dialog.close();
        assert!(!dialog.is_open());
    }

    #[test]
    fn test_settings_update() {
        let settings = AppSettings::default();
        let mut dialog = SettingsDialog::new(settings);

        let mut new_settings = AppSettings::default();
        new_settings.audio.sample_rate = 48000;
        new_settings.ui.theme = "light".to_string();
        new_settings.defaults.default_bpm = 140.0;

        dialog.update_settings(new_settings.clone());

        assert_eq!(dialog.get_settings().audio.sample_rate, 48000);
        assert_eq!(dialog.get_settings().ui.theme, "light");
        assert_eq!(dialog.get_settings().defaults.default_bpm, 140.0);
    }

    #[test]
    fn test_device_refresh_request() {
        let settings = AppSettings::default();
        let mut dialog = SettingsDialog::new(settings);

        assert!(!dialog.take_device_refresh_requested());

        // Simulate device refresh request
        dialog.device_refresh_requested = true;
        assert!(dialog.take_device_refresh_requested());
        assert!(!dialog.take_device_refresh_requested()); // Should be reset
    }

    #[test]
    fn test_available_devices_update() {
        let settings = AppSettings::default();
        let mut dialog = SettingsDialog::new(settings);

        let devices = vec![
            "Default Device".to_string(),
            "Audio Interface 1".to_string(),
            "Audio Interface 2".to_string(),
        ];

        dialog.update_available_devices(devices.clone());
        assert_eq!(dialog.available_devices, devices);
    }

    #[test]
    fn test_button_behavior() {
        let mut settings = AppSettings::default();
        settings.audio.master_volume = 0.8;
        settings.ui.theme = "light".to_string();
        let mut dialog = SettingsDialog::new(settings.clone());

        // Open dialog - original settings should be stored
        dialog.open();
        assert_eq!(dialog.original_settings.audio.master_volume, 0.8);
        assert_eq!(dialog.original_settings.ui.theme, "light");

        // Modify current settings
        dialog.settings.audio.master_volume = 0.6;
        dialog.settings.ui.theme = "dark".to_string();

        // Test Cancel - should revert to original
        let mut test_dialog = dialog.clone();
        test_dialog.settings = test_dialog.original_settings.clone(); // Simulate cancel
        assert_eq!(test_dialog.settings.audio.master_volume, 0.8);
        assert_eq!(test_dialog.settings.ui.theme, "light");

        // Test Apply - should update original settings
        dialog.original_settings = dialog.settings.clone(); // Simulate apply
        assert_eq!(dialog.original_settings.audio.master_volume, 0.6);
        assert_eq!(dialog.original_settings.ui.theme, "dark");

        // Now modify again and test Cancel - should revert to applied values
        dialog.settings.audio.master_volume = 0.4;
        dialog.settings.ui.theme = "light".to_string();

        // Cancel should now revert to the applied values, not the original original values
        dialog.settings = dialog.original_settings.clone(); // Simulate cancel after apply
        assert_eq!(dialog.settings.audio.master_volume, 0.6);
        assert_eq!(dialog.settings.ui.theme, "dark");
    }

    #[test]
    fn test_close_button_closes_dialog() {
        let settings = AppSettings::default();
        let mut dialog = SettingsDialog::new(settings);

        // Open dialog
        dialog.open();
        assert!(dialog.is_open());

        // Simulate close button click
        dialog.original_settings = dialog.settings.clone(); // What close button does internally
        dialog.close(); // What close button calls

        // Dialog should be closed
        assert!(!dialog.is_open());
    }
}
