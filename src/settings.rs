use anyhow::Result;
use eframe::egui;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Audio settings for the application
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioSettings {
    pub sample_rate: u32,
    pub buffer_size: u32,
    pub master_volume: f32,
    pub preferred_device: Option<String>,
    pub device_monitoring_enabled: bool,
    pub auto_fallback_enabled: bool,
    pub last_known_good_device: Option<String>,
}

impl Default for AudioSettings {
    fn default() -> Self {
        AudioSettings {
            sample_rate: 44100,
            buffer_size: 1024,
            master_volume: 1.0,
            preferred_device: None,
            device_monitoring_enabled: true,
            auto_fallback_enabled: true,
            last_known_good_device: None,
        }
    }
}

impl AudioSettings {
    /// Validate audio settings values
    pub fn validate(&self) -> Result<()> {
        if self.sample_rate < 22050 || self.sample_rate > 192000 {
            return Err(anyhow::anyhow!(
                "Sample rate must be between 22050 and 192000 Hz"
            ));
        }

        if ![64, 128, 256, 512, 1024, 2048, 4096].contains(&self.buffer_size) {
            return Err(anyhow::anyhow!(
                "Buffer size must be a power of 2 between 64 and 4096"
            ));
        }

        if self.master_volume < 0.0 || self.master_volume > 2.0 {
            return Err(anyhow::anyhow!("Master volume must be between 0.0 and 2.0"));
        }

        Ok(())
    }
}

/// UI settings for the application
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UISettings {
    pub theme: String,
    pub ui_scale: f32,
}

impl Default for UISettings {
    fn default() -> Self {
        // Set default scale based on platform (2x for macOS, 1x for others)
        let default_scale = if cfg!(target_os = "macos") { 2.0 } else { 1.0 };

        UISettings {
            theme: "dark".to_string(),
            ui_scale: default_scale,
        }
    }
}

impl UISettings {
    /// Validate UI settings values
    pub fn validate(&self) -> Result<()> {
        // Validate theme
        if !["dark", "light", "auto"].contains(&self.theme.as_str()) {
            return Err(anyhow::anyhow!(
                "Theme must be 'dark', 'light', or 'auto', got '{}'",
                self.theme
            ));
        }

        // Check for NaN or infinite values first (before range checks)
        if !self.ui_scale.is_finite() {
            return Err(anyhow::anyhow!(
                "UI scale must be a finite number, got {}",
                self.ui_scale
            ));
        }

        // Validate scale range with more detailed error information
        if self.ui_scale < 0.5 {
            return Err(anyhow::anyhow!(
                "UI scale {} is too small (minimum: 0.5)",
                self.ui_scale
            ));
        }
        if self.ui_scale > 3.0 {
            return Err(anyhow::anyhow!(
                "UI scale {} is too large (maximum: 3.0)",
                self.ui_scale
            ));
        }

        Ok(())
    }

    /// Sanitize individual UI settings values, correcting invalid ones
    pub fn sanitize(&mut self) -> Vec<String> {
        let mut corrections = Vec::new();

        // Sanitize theme
        if !["dark", "light", "auto"].contains(&self.theme.as_str()) {
            corrections.push(format!(
                "Theme '{}' is invalid, changed to 'dark'",
                self.theme
            ));
            self.theme = "dark".to_string();
        }

        // Sanitize scale (check finite first, then range)
        if !self.ui_scale.is_finite() {
            let default_scale = UISettings::default().ui_scale;
            corrections.push(format!(
                "UI scale {} is invalid, changed to {}",
                self.ui_scale, default_scale
            ));
            self.ui_scale = default_scale;
        } else if self.ui_scale < 0.5 {
            corrections.push(format!(
                "UI scale {} is too small, changed to 0.5",
                self.ui_scale
            ));
            self.ui_scale = 0.5;
        } else if self.ui_scale > 3.0 {
            corrections.push(format!(
                "UI scale {} is too large, changed to 3.0",
                self.ui_scale
            ));
            self.ui_scale = 3.0;
        }

        corrections
    }

    /// Detect system theme preference
    #[cfg(not(target_arch = "wasm32"))]
    pub fn detect_system_theme() -> String {
        match dark_light::detect() {
            dark_light::Mode::Dark => "dark".to_string(),
            dark_light::Mode::Light => "light".to_string(),
            dark_light::Mode::Default => "dark".to_string(), // fallback to dark
        }
    }

    /// Detect system theme preference (WASM fallback)
    #[cfg(target_arch = "wasm32")]
    pub fn detect_system_theme() -> String {
        // For WASM builds, we fallback to dark theme
        // TODO: Could implement browser-based detection via web APIs
        "dark".to_string()
    }

    /// Resolve the actual theme to use based on settings
    pub fn resolve_theme(&self) -> String {
        match self.theme.as_str() {
            "auto" => Self::detect_system_theme(),
            "dark" | "light" => self.theme.clone(),
            _ => "dark".to_string(), // fallback for invalid themes
        }
    }
}

/// Keyboard shortcut settings for the application
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyboardSettings {
    // Transport shortcuts
    pub play_pause: String,
    pub return_to_start: String,
    pub stop_escape: String,

    // Timeline navigation
    pub timeline_step_back: String,
    pub timeline_step_forward: String,
    pub timeline_jump_back: String,
    pub timeline_jump_forward: String,
    pub timeline_start: String,
    pub timeline_end: String,

    // Pattern editing
    pub pattern_clear: String,
    pub pattern_select_all: String,

    // Application shortcuts
    pub new_project: String,
    pub open_project: String,
    pub save_project: String,
    pub save_project_as: String,
    pub open_settings: String,
}

impl Default for KeyboardSettings {
    fn default() -> Self {
        // Platform-specific modifier key for application shortcuts
        let primary_modifier = if cfg!(target_os = "macos") {
            "Cmd"
        } else {
            "Ctrl"
        };

        KeyboardSettings {
            // Transport shortcuts
            play_pause: "Space".to_string(),
            return_to_start: "Enter".to_string(),
            stop_escape: "Escape".to_string(),

            // Timeline navigation
            timeline_step_back: "Left".to_string(),
            timeline_step_forward: "Right".to_string(),
            timeline_jump_back: "Shift+Left".to_string(),
            timeline_jump_forward: "Shift+Right".to_string(),
            timeline_start: "Home".to_string(),
            timeline_end: "End".to_string(),

            // Pattern editing
            pattern_clear: "Delete".to_string(),
            pattern_select_all: format!("{}+A", primary_modifier),

            // Application shortcuts
            new_project: format!("{}+N", primary_modifier),
            open_project: format!("{}+O", primary_modifier),
            save_project: format!("{}+S", primary_modifier),
            save_project_as: format!("{}+Shift+S", primary_modifier),
            open_settings: format!("{}+,", primary_modifier),
        }
    }
}

impl KeyboardSettings {
    /// Validate keyboard settings values
    pub fn validate(&self) -> Result<()> {
        let shortcuts = [
            &self.play_pause,
            &self.return_to_start,
            &self.stop_escape,
            &self.timeline_step_back,
            &self.timeline_step_forward,
            &self.timeline_jump_back,
            &self.timeline_jump_forward,
            &self.timeline_start,
            &self.timeline_end,
            &self.pattern_clear,
            &self.pattern_select_all,
            &self.new_project,
            &self.open_project,
            &self.save_project,
            &self.save_project_as,
            &self.open_settings,
        ];

        // Validate each shortcut string format
        for shortcut in shortcuts {
            if shortcut.trim().is_empty() {
                return Err(anyhow::anyhow!("Keyboard shortcut cannot be empty"));
            }

            // Basic validation for modifier+key format
            if shortcut.contains('+') {
                let parts: Vec<&str> = shortcut.split('+').collect();
                if parts.len() > 3 {
                    return Err(anyhow::anyhow!(
                        "Invalid shortcut format '{}': too many modifiers", 
                        shortcut
                    ));
                }
                for part in parts {
                    if part.trim().is_empty() {
                        return Err(anyhow::anyhow!(
                            "Invalid shortcut format '{}': empty modifier or key",
                            shortcut
                        ));
                    }
                }
            }
        }

        // Check for duplicate shortcuts
        let mut used_shortcuts = std::collections::HashSet::new();
        for shortcut in shortcuts {
            if used_shortcuts.contains(shortcut) {
                return Err(anyhow::anyhow!(
                    "Duplicate keyboard shortcut: '{}'", 
                    shortcut
                ));
            }
            used_shortcuts.insert(shortcut);
        }

        Ok(())
    }

    /// Sanitize keyboard settings by correcting invalid values
    pub fn sanitize(&mut self) -> Vec<String> {
        let mut corrections = Vec::new();
        let defaults = KeyboardSettings::default();

        // Helper function to sanitize a single shortcut
        let mut sanitize_shortcut = |current: &mut String, default: &str, name: &str| {
            if current.trim().is_empty() {
                corrections.push(format!(
                    "Keyboard shortcut '{}' was empty, changed to '{}'",
                    name, default
                ));
                *current = default.to_string();
            }
        };

        // Sanitize each shortcut
        sanitize_shortcut(&mut self.play_pause, &defaults.play_pause, "Play/Pause");
        sanitize_shortcut(&mut self.return_to_start, &defaults.return_to_start, "Return to Start");
        sanitize_shortcut(&mut self.stop_escape, &defaults.stop_escape, "Stop/Escape");
        sanitize_shortcut(&mut self.timeline_step_back, &defaults.timeline_step_back, "Timeline Step Back");
        sanitize_shortcut(&mut self.timeline_step_forward, &defaults.timeline_step_forward, "Timeline Step Forward");
        sanitize_shortcut(&mut self.timeline_jump_back, &defaults.timeline_jump_back, "Timeline Jump Back");
        sanitize_shortcut(&mut self.timeline_jump_forward, &defaults.timeline_jump_forward, "Timeline Jump Forward");
        sanitize_shortcut(&mut self.timeline_start, &defaults.timeline_start, "Timeline Start");
        sanitize_shortcut(&mut self.timeline_end, &defaults.timeline_end, "Timeline End");
        sanitize_shortcut(&mut self.pattern_clear, &defaults.pattern_clear, "Pattern Clear");
        sanitize_shortcut(&mut self.pattern_select_all, &defaults.pattern_select_all, "Pattern Select All");
        sanitize_shortcut(&mut self.new_project, &defaults.new_project, "New Project");
        sanitize_shortcut(&mut self.open_project, &defaults.open_project, "Open Project");
        sanitize_shortcut(&mut self.save_project, &defaults.save_project, "Save Project");
        sanitize_shortcut(&mut self.save_project_as, &defaults.save_project_as, "Save Project As");
        sanitize_shortcut(&mut self.open_settings, &defaults.open_settings, "Open Settings");

        corrections
    }

    /// Parse a shortcut string into egui key and modifiers
    pub fn parse_shortcut(shortcut: &str) -> Option<(egui::Key, egui::Modifiers)> {
        if shortcut.trim().is_empty() {
            return None;
        }

        let parts: Vec<&str> = shortcut.split('+').map(|s| s.trim()).collect();
        if parts.is_empty() {
            return None;
        }

        let key_str = parts.last()?;
        let key = match key_str.to_lowercase().as_str() {
            "space" => egui::Key::Space,
            "enter" => egui::Key::Enter,
            "escape" => egui::Key::Escape,
            "left" => egui::Key::ArrowLeft,
            "right" => egui::Key::ArrowRight,
            "home" => egui::Key::Home,
            "end" => egui::Key::End,
            "delete" => egui::Key::Delete,
            "backspace" => egui::Key::Backspace,
            "tab" => egui::Key::Tab,
            "a" => egui::Key::A,
            "n" => egui::Key::N,
            "o" => egui::Key::O,
            "s" => egui::Key::S,
            "comma" => egui::Key::Comma,
            "," => egui::Key::Comma,
            "." => egui::Key::Period,
            ";" => egui::Key::Semicolon,
            _ => return None,
        };

        let mut modifiers = egui::Modifiers::default();
        for modifier in &parts[..parts.len() - 1] {
            match modifier.to_lowercase().as_str() {
                "ctrl" => modifiers.ctrl = true,
                "cmd" => modifiers.mac_cmd = true,
                "alt" => modifiers.alt = true,
                "shift" => modifiers.shift = true,
                _ => return None,
            }
        }

        Some((key, modifiers))
    }

    /// Check if a key press matches a shortcut
    pub fn matches_shortcut(
        shortcut: &str, 
        key: egui::Key, 
        modifiers: &egui::Modifiers
    ) -> bool {
        if let Some((expected_key, expected_modifiers)) = Self::parse_shortcut(shortcut) {
            key == expected_key && modifiers.matches_exact(expected_modifiers)
        } else {
            false
        }
    }
}

/// Default settings for new projects
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefaultSettings {
    pub default_bpm: f32,
    pub default_time_signature: (u32, u32),
    pub default_pattern_length: usize,
}

impl Default for DefaultSettings {
    fn default() -> Self {
        DefaultSettings {
            default_bpm: 120.0,
            default_time_signature: (4, 4),
            default_pattern_length: 16,
        }
    }
}

impl DefaultSettings {
    /// Validate default settings values with detailed error messages
    pub fn validate(&self) -> Result<()> {
        // Check for NaN or infinite values first
        if !self.default_bpm.is_finite() {
            return Err(anyhow::anyhow!(
                "Default BPM must be a finite number, got {}",
                self.default_bpm
            ));
        }

        // Validate BPM range with detailed error information
        if self.default_bpm < 60.0 {
            return Err(anyhow::anyhow!(
                "Default BPM {} is too slow (minimum: 60 BPM)",
                self.default_bpm
            ));
        }
        if self.default_bpm > 300.0 {
            return Err(anyhow::anyhow!(
                "Default BPM {} is too fast (maximum: 300 BPM)",
                self.default_bpm
            ));
        }

        let (numerator, denominator) = self.default_time_signature;

        // Validate time signature numerator
        if numerator < 1 {
            return Err(anyhow::anyhow!(
                "Time signature numerator {} is too small (minimum: 1)",
                numerator
            ));
        }
        if numerator > 16 {
            return Err(anyhow::anyhow!(
                "Time signature numerator {} is too large (maximum: 16)",
                numerator
            ));
        }

        // Validate time signature denominator with detailed error
        if ![1, 2, 4, 8, 16].contains(&denominator) {
            return Err(anyhow::anyhow!(
                "Time signature denominator {} is invalid (must be 1, 2, 4, 8, or 16)",
                denominator
            ));
        }

        // Validate pattern length with detailed error information
        if self.default_pattern_length < 4 {
            return Err(anyhow::anyhow!(
                "Default pattern length {} is too short (minimum: 4 steps)",
                self.default_pattern_length
            ));
        }
        if self.default_pattern_length > 64 {
            return Err(anyhow::anyhow!(
                "Default pattern length {} is too long (maximum: 64 steps)",
                self.default_pattern_length
            ));
        }

        Ok(())
    }

    /// Sanitize default settings values, correcting invalid ones
    pub fn sanitize(&mut self) -> Vec<String> {
        let mut corrections = Vec::new();

        // Sanitize BPM (check finite first, then range)
        if !self.default_bpm.is_finite() {
            let default_bpm = DefaultSettings::default().default_bpm;
            corrections.push(format!(
                "Default BPM {} is invalid, changed to {}",
                self.default_bpm, default_bpm
            ));
            self.default_bpm = default_bpm;
        } else if self.default_bpm < 60.0 {
            corrections.push(format!(
                "Default BPM {} is too slow, changed to 60",
                self.default_bpm
            ));
            self.default_bpm = 60.0;
        } else if self.default_bpm > 300.0 {
            corrections.push(format!(
                "Default BPM {} is too fast, changed to 300",
                self.default_bpm
            ));
            self.default_bpm = 300.0;
        }

        // Sanitize time signature
        let (mut numerator, mut denominator) = self.default_time_signature;
        let original_time_sig = (numerator, denominator);

        // Sanitize numerator
        if numerator < 1 {
            corrections.push(format!(
                "Time signature numerator {} is too small, changed to 1",
                numerator
            ));
            numerator = 1;
        } else if numerator > 16 {
            corrections.push(format!(
                "Time signature numerator {} is too large, changed to 16",
                numerator
            ));
            numerator = 16;
        }

        // Sanitize denominator
        if ![1, 2, 4, 8, 16].contains(&denominator) {
            corrections.push(format!(
                "Time signature denominator {} is invalid, changed to 4",
                denominator
            ));
            denominator = 4;
        }

        // Update time signature if it was modified
        if (numerator, denominator) != original_time_sig {
            self.default_time_signature = (numerator, denominator);
        }

        // Sanitize pattern length
        if self.default_pattern_length < 4 {
            corrections.push(format!(
                "Default pattern length {} is too short, changed to 4",
                self.default_pattern_length
            ));
            self.default_pattern_length = 4;
        } else if self.default_pattern_length > 64 {
            corrections.push(format!(
                "Default pattern length {} is too long, changed to 64",
                self.default_pattern_length
            ));
            self.default_pattern_length = 64;
        }

        corrections
    }
}

/// Main application settings structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub audio: AudioSettings,
    pub ui: UISettings,
    pub defaults: DefaultSettings,
    pub keyboard: KeyboardSettings,
}

impl Default for AppSettings {
    fn default() -> Self {
        AppSettings {
            audio: AudioSettings::default(),
            ui: UISettings::default(),
            defaults: DefaultSettings::default(),
            keyboard: KeyboardSettings::default(),
        }
    }
}

impl AppSettings {
    /// Validate all settings
    pub fn validate(&self) -> Result<()> {
        self.audio.validate()?;
        self.ui.validate()?;
        self.defaults.validate()?;
        self.keyboard.validate()?;
        Ok(())
    }

    /// Get the settings file path for the current platform
    pub fn get_settings_file_path() -> Result<PathBuf> {
        #[cfg(target_arch = "wasm32")]
        {
            // For WASM builds, settings would be stored in local storage
            // Return a dummy path for now
            Ok(PathBuf::from("beatr_settings.json"))
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let config_dir = dirs::config_dir()
                .ok_or_else(|| anyhow::anyhow!("Could not determine config directory"))?;

            let app_config_dir = config_dir.join("beatr");
            std::fs::create_dir_all(&app_config_dir)?;

            Ok(app_config_dir.join("settings.json"))
        }
    }

    /// Save settings to file
    pub fn save_to_file(&self) -> Result<()> {
        let path = Self::get_settings_file_path()?;

        #[cfg(target_arch = "wasm32")]
        {
            // For WASM, would use web_sys to save to localStorage
            // For now, just return OK as we'll implement this later
            Ok(())
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let json = serde_json::to_string_pretty(self)?;
            std::fs::write(path, json)?;
            Ok(())
        }
    }

    /// Load settings from file, fallback to defaults if file doesn't exist or is invalid
    pub fn load_from_file() -> Self {
        #[cfg(target_arch = "wasm32")]
        {
            // For WASM, would use web_sys to load from localStorage
            // For now, just return defaults
            Self::default()
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            match Self::get_settings_file_path() {
                Ok(path) => {
                    if path.exists() {
                        match std::fs::read_to_string(&path) {
                            Ok(content) => {
                                match serde_json::from_str::<AppSettings>(&content) {
                                    Ok(mut settings) => {
                                        // Validate loaded settings, fallback to defaults for invalid values
                                        if settings.validate().is_err() {
                                            eprintln!("Warning: Invalid settings detected, using defaults for invalid values");
                                            settings = Self::sanitize_settings(settings);
                                        }
                                        settings
                                    }
                                    Err(err) => {
                                        eprintln!("Warning: Failed to parse settings file: {}. Using defaults.", err);
                                        Self::default()
                                    }
                                }
                            }
                            Err(err) => {
                                eprintln!(
                                    "Warning: Failed to read settings file: {}. Using defaults.",
                                    err
                                );
                                Self::default()
                            }
                        }
                    } else {
                        // File doesn't exist, use defaults and save them
                        let defaults = Self::default();
                        if let Err(err) = defaults.save_to_file() {
                            eprintln!("Warning: Failed to save default settings: {}", err);
                        }
                        defaults
                    }
                }
                Err(err) => {
                    eprintln!(
                        "Warning: Failed to determine settings file path: {}. Using defaults.",
                        err
                    );
                    Self::default()
                }
            }
        }
    }

    /// Sanitize settings by correcting invalid values and providing feedback
    fn sanitize_settings(mut settings: AppSettings) -> Self {
        let mut all_corrections = Vec::new();

        // Sanitize audio settings (use coarse-grained approach for now)
        if settings.audio.validate().is_err() {
            all_corrections.push("Audio settings were invalid and reset to defaults".to_string());
            settings.audio = AudioSettings::default();
        }

        // Sanitize UI settings with granular feedback
        let ui_corrections = settings.ui.sanitize();
        all_corrections.extend(ui_corrections);

        // Sanitize default settings with granular feedback
        let default_corrections = settings.defaults.sanitize();
        all_corrections.extend(default_corrections);

        // Sanitize keyboard settings with granular feedback
        let keyboard_corrections = settings.keyboard.sanitize();
        all_corrections.extend(keyboard_corrections);

        // Report corrections if any were made
        if !all_corrections.is_empty() {
            eprintln!("Settings corrections made:");
            for correction in all_corrections {
                eprintln!("  - {}", correction);
            }
        }

        settings
    }

    /// Auto-save settings after changes (for immediate apply functionality)
    pub fn auto_save(&self) -> Result<()> {
        self.validate()?;
        self.save_to_file()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_audio_settings_validation() {
        let mut settings = AudioSettings::default();
        assert!(settings.validate().is_ok());

        // Test invalid sample rate
        settings.sample_rate = 10000;
        assert!(settings.validate().is_err());

        // Reset and test invalid buffer size
        settings.sample_rate = 44100;
        settings.buffer_size = 1000;
        assert!(settings.validate().is_err());

        // Reset and test invalid volume
        settings.buffer_size = 1024;
        settings.master_volume = -1.0;
        assert!(settings.validate().is_err());
    }

    #[test]
    fn test_ui_settings_validation() {
        let mut settings = UISettings::default();
        assert!(settings.validate().is_ok());

        // Test valid themes
        settings.theme = "light".to_string();
        assert!(settings.validate().is_ok());

        settings.theme = "auto".to_string();
        assert!(settings.validate().is_ok());

        // Test invalid theme
        settings.theme = "invalid".to_string();
        assert!(settings.validate().is_err());

        // Reset and test invalid scale
        settings.theme = "dark".to_string();
        settings.ui_scale = 5.0;
        assert!(settings.validate().is_err());

        // Test passes now that window size validation is removed
        settings.ui_scale = 1.0;
        assert!(settings.validate().is_ok());
    }

    #[test]
    fn test_theme_resolution() {
        let mut settings = UISettings::default();

        // Test direct theme resolution
        settings.theme = "dark".to_string();
        assert_eq!(settings.resolve_theme(), "dark");

        settings.theme = "light".to_string();
        assert_eq!(settings.resolve_theme(), "light");

        // Test auto theme resolution (should return either "dark" or "light")
        settings.theme = "auto".to_string();
        let resolved = settings.resolve_theme();
        assert!(resolved == "dark" || resolved == "light");

        // Test invalid theme fallback
        settings.theme = "invalid".to_string();
        assert_eq!(settings.resolve_theme(), "dark");
    }

    #[test]
    fn test_system_theme_detection() {
        // Test that system theme detection returns a valid theme
        let detected = UISettings::detect_system_theme();
        assert!(detected == "dark" || detected == "light");
    }

    #[test]
    fn test_platform_specific_defaults() {
        let settings = UISettings::default();

        // Test that scale default is platform-specific
        if cfg!(target_os = "macos") {
            assert_eq!(settings.ui_scale, 2.0);
        } else {
            assert_eq!(settings.ui_scale, 1.0);
        }

        // Theme should always default to dark
        assert_eq!(settings.theme, "dark");

        // Validation should pass with defaults
        assert!(settings.validate().is_ok());
    }

    #[test]
    fn test_enhanced_ui_validation_errors() {
        let mut settings = UISettings::default();

        // Test invalid theme with detailed error
        settings.theme = "purple".to_string();
        let error = settings.validate().unwrap_err();
        assert!(error.to_string().contains("purple"));
        assert!(error.to_string().contains("'dark', 'light', or 'auto'"));

        // Test scale too small
        settings.theme = "dark".to_string();
        settings.ui_scale = 0.3;
        let error = settings.validate().unwrap_err();
        assert!(error.to_string().contains("0.3"));
        assert!(error.to_string().contains("too small"));

        // Test scale too large
        settings.ui_scale = 5.0;
        let error = settings.validate().unwrap_err();
        assert!(error.to_string().contains("5"));
        assert!(error.to_string().contains("too large"));

        // Test NaN scale
        settings.ui_scale = f32::NAN;
        let error = settings.validate().unwrap_err();
        assert!(error.to_string().contains("finite"));

        // Test infinity scale
        settings.ui_scale = f32::INFINITY;
        let error = settings.validate().unwrap_err();
        assert!(error.to_string().contains("finite"));
    }

    #[test]
    fn test_ui_settings_sanitization() {
        let mut settings = UISettings::default();

        // Test invalid theme sanitization
        settings.theme = "invalid".to_string();
        settings.ui_scale = 1.0;
        let corrections = settings.sanitize();
        assert_eq!(settings.theme, "dark");
        assert_eq!(corrections.len(), 1);
        assert!(corrections[0].contains("invalid"));

        // Test scale too small sanitization
        settings = UISettings::default();
        settings.ui_scale = 0.3;
        let corrections = settings.sanitize();
        assert_eq!(settings.ui_scale, 0.5);
        assert_eq!(corrections.len(), 1);
        assert!(corrections[0].contains("0.3"));

        // Test scale too large sanitization
        settings = UISettings::default();
        settings.ui_scale = 5.0;
        let corrections = settings.sanitize();
        assert_eq!(settings.ui_scale, 3.0);
        assert_eq!(corrections.len(), 1);
        assert!(corrections[0].contains("5"));

        // Test NaN sanitization
        settings = UISettings::default();
        settings.ui_scale = f32::NAN;
        let corrections = settings.sanitize();
        let expected_default = UISettings::default().ui_scale;
        assert_eq!(settings.ui_scale, expected_default);
        assert_eq!(corrections.len(), 1);
        assert!(corrections[0].contains("invalid"));

        // Test multiple corrections
        settings = UISettings {
            theme: "purple".to_string(),
            ui_scale: 10.0,
        };
        let corrections = settings.sanitize();
        assert_eq!(settings.theme, "dark");
        assert_eq!(settings.ui_scale, 3.0);
        assert_eq!(corrections.len(), 2);
    }

    #[test]
    fn test_default_settings_validation() {
        let mut settings = DefaultSettings::default();
        assert!(settings.validate().is_ok());

        // Test invalid BPM
        settings.default_bpm = 500.0;
        assert!(settings.validate().is_err());

        // Reset and test invalid time signature
        settings.default_bpm = 120.0;
        settings.default_time_signature = (20, 4);
        assert!(settings.validate().is_err());

        // Reset and test invalid pattern length
        settings.default_time_signature = (4, 4);
        settings.default_pattern_length = 100;
        assert!(settings.validate().is_err());
    }

    #[test]
    fn test_enhanced_default_settings_validation_errors() {
        let mut settings = DefaultSettings::default();

        // Test BPM too slow with detailed error
        settings.default_bpm = 30.0;
        let error = settings.validate().unwrap_err();
        assert!(error.to_string().contains("30"));
        assert!(error.to_string().contains("too slow"));
        assert!(error.to_string().contains("minimum: 60"));

        // Test BPM too fast
        settings.default_bpm = 400.0;
        let error = settings.validate().unwrap_err();
        assert!(error.to_string().contains("400"));
        assert!(error.to_string().contains("too fast"));
        assert!(error.to_string().contains("maximum: 300"));

        // Test NaN BPM
        settings.default_bpm = f32::NAN;
        let error = settings.validate().unwrap_err();
        assert!(error.to_string().contains("finite"));

        // Test infinity BPM
        settings.default_bpm = f32::INFINITY;
        let error = settings.validate().unwrap_err();
        assert!(error.to_string().contains("finite"));

        // Reset BPM and test time signature numerator too small
        settings.default_bpm = 120.0;
        settings.default_time_signature = (0, 4);
        let error = settings.validate().unwrap_err();
        assert!(error.to_string().contains("numerator 0"));
        assert!(error.to_string().contains("too small"));

        // Test time signature numerator too large
        settings.default_time_signature = (20, 4);
        let error = settings.validate().unwrap_err();
        assert!(error.to_string().contains("numerator 20"));
        assert!(error.to_string().contains("too large"));

        // Test invalid time signature denominator
        settings.default_time_signature = (4, 3);
        let error = settings.validate().unwrap_err();
        assert!(error.to_string().contains("denominator 3"));
        assert!(error.to_string().contains("invalid"));
        assert!(error.to_string().contains("must be 1, 2, 4, 8, or 16"));

        // Reset time signature and test pattern length too short
        settings.default_time_signature = (4, 4);
        settings.default_pattern_length = 2;
        let error = settings.validate().unwrap_err();
        assert!(error.to_string().contains("length 2"));
        assert!(error.to_string().contains("too short"));
        assert!(error.to_string().contains("minimum: 4"));

        // Test pattern length too long
        settings.default_pattern_length = 100;
        let error = settings.validate().unwrap_err();
        assert!(error.to_string().contains("length 100"));
        assert!(error.to_string().contains("too long"));
        assert!(error.to_string().contains("maximum: 64"));
    }

    #[test]
    fn test_default_settings_sanitization() {
        let mut settings = DefaultSettings::default();

        // Test BPM too small sanitization
        settings.default_bpm = 30.0;
        let corrections = settings.sanitize();
        assert_eq!(settings.default_bpm, 60.0);
        assert_eq!(corrections.len(), 1);
        assert!(corrections[0].contains("30"));
        assert!(corrections[0].contains("too slow"));

        // Test BPM too large sanitization
        settings = DefaultSettings::default();
        settings.default_bpm = 400.0;
        let corrections = settings.sanitize();
        assert_eq!(settings.default_bpm, 300.0);
        assert_eq!(corrections.len(), 1);
        assert!(corrections[0].contains("400"));
        assert!(corrections[0].contains("too fast"));

        // Test NaN sanitization
        settings = DefaultSettings::default();
        settings.default_bpm = f32::NAN;
        let corrections = settings.sanitize();
        let expected_default = DefaultSettings::default().default_bpm;
        assert_eq!(settings.default_bpm, expected_default);
        assert_eq!(corrections.len(), 1);
        assert!(corrections[0].contains("invalid"));

        // Test time signature numerator sanitization
        settings = DefaultSettings::default();
        settings.default_time_signature = (0, 4);
        let corrections = settings.sanitize();
        assert_eq!(settings.default_time_signature.0, 1);
        assert_eq!(corrections.len(), 1);
        assert!(corrections[0].contains("numerator 0"));
        assert!(corrections[0].contains("too small"));

        settings = DefaultSettings::default();
        settings.default_time_signature = (20, 4);
        let corrections = settings.sanitize();
        assert_eq!(settings.default_time_signature.0, 16);
        assert_eq!(corrections.len(), 1);
        assert!(corrections[0].contains("numerator 20"));
        assert!(corrections[0].contains("too large"));

        // Test time signature denominator sanitization
        settings = DefaultSettings::default();
        settings.default_time_signature = (4, 3);
        let corrections = settings.sanitize();
        assert_eq!(settings.default_time_signature.1, 4);
        assert_eq!(corrections.len(), 1);
        assert!(corrections[0].contains("denominator 3"));
        assert!(corrections[0].contains("invalid"));

        // Test pattern length sanitization
        settings = DefaultSettings::default();
        settings.default_pattern_length = 2;
        let corrections = settings.sanitize();
        assert_eq!(settings.default_pattern_length, 4);
        assert_eq!(corrections.len(), 1);
        assert!(corrections[0].contains("length 2"));
        assert!(corrections[0].contains("too short"));

        settings = DefaultSettings::default();
        settings.default_pattern_length = 100;
        let corrections = settings.sanitize();
        assert_eq!(settings.default_pattern_length, 64);
        assert_eq!(corrections.len(), 1);
        assert!(corrections[0].contains("length 100"));
        assert!(corrections[0].contains("too long"));

        // Test multiple corrections
        settings = DefaultSettings {
            default_bpm: 500.0,
            default_time_signature: (0, 3),
            default_pattern_length: 100,
        };
        let corrections = settings.sanitize();
        assert_eq!(settings.default_bpm, 300.0);
        assert_eq!(settings.default_time_signature, (1, 4));
        assert_eq!(settings.default_pattern_length, 64);
        assert_eq!(corrections.len(), 4); // BPM, numerator, denominator, pattern length
    }

    #[test]
    fn test_app_settings_creation_and_validation() {
        let default_settings = AppSettings::default();
        assert!(default_settings.validate().is_ok());
    }

    #[test]
    fn test_settings_serialization() {
        let original_settings = AppSettings::default();

        let json = serde_json::to_string_pretty(&original_settings).unwrap();
        let deserialized_settings: AppSettings = serde_json::from_str(&json).unwrap();

        assert!(deserialized_settings.validate().is_ok());
        assert_eq!(
            original_settings.audio.sample_rate,
            deserialized_settings.audio.sample_rate
        );
        assert_eq!(original_settings.ui.theme, deserialized_settings.ui.theme);
        assert_eq!(
            original_settings.defaults.default_bpm,
            deserialized_settings.defaults.default_bpm
        );
    }

    #[test]
    fn test_settings_sanitization() {
        let mut invalid_settings = AppSettings::default();
        invalid_settings.audio.sample_rate = 10000; // Invalid
        invalid_settings.ui.theme = "invalid".to_string(); // Invalid
        invalid_settings.defaults.default_bpm = 500.0; // Invalid

        let sanitized = AppSettings::sanitize_settings(invalid_settings);
        assert!(sanitized.validate().is_ok());
        assert_eq!(
            sanitized.audio.sample_rate,
            AudioSettings::default().sample_rate
        );
        assert_eq!(sanitized.ui.theme, UISettings::default().theme);
        assert_eq!(sanitized.defaults.default_bpm, 300.0); // Should be sanitized to max value, not default
    }

    #[test]
    fn test_auto_save_validation() {
        let mut settings = AppSettings::default();

        // Test auto-save with valid settings
        assert!(settings.auto_save().is_ok());

        // Test auto-save with invalid settings
        settings.audio.sample_rate = 10000; // Invalid
        assert!(
            settings.auto_save().is_err(),
            "Auto-save should fail with invalid settings"
        );

        // Sanitize and retry
        settings.audio = AudioSettings::default();
        assert!(settings.auto_save().is_ok());
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn test_settings_file_operations() {
        // This test uses a temp directory to avoid interfering with real settings
        let temp_dir = tempdir().unwrap();
        let temp_settings_path = temp_dir.path().join("test_settings.json");

        // Create test settings
        let mut test_settings = AppSettings::default();
        test_settings.audio.sample_rate = 48000;
        test_settings.ui.theme = "light".to_string();
        test_settings.defaults.default_bpm = 140.0;

        // Save to temp file manually (since we can't override get_settings_file_path easily)
        let json = serde_json::to_string_pretty(&test_settings).unwrap();
        std::fs::write(&temp_settings_path, json).unwrap();

        // Load from temp file manually
        let content = std::fs::read_to_string(&temp_settings_path).unwrap();
        let loaded_settings: AppSettings = serde_json::from_str(&content).unwrap();

        assert!(loaded_settings.validate().is_ok());
        assert_eq!(loaded_settings.audio.sample_rate, 48000);
        assert_eq!(loaded_settings.ui.theme, "light");
        assert_eq!(loaded_settings.defaults.default_bpm, 140.0);
    }

    #[test]
    fn test_keyboard_settings_validation() {
        let settings = KeyboardSettings::default();
        assert!(settings.validate().is_ok());

        // Test with empty shortcut
        let mut invalid_settings = settings.clone();
        invalid_settings.play_pause = "".to_string();
        assert!(invalid_settings.validate().is_err());

        // Test with duplicate shortcuts
        let mut duplicate_settings = settings.clone();
        duplicate_settings.return_to_start = duplicate_settings.play_pause.clone();
        assert!(duplicate_settings.validate().is_err());

        // Test with too many modifiers
        let mut complex_settings = settings.clone();
        complex_settings.play_pause = "Ctrl+Alt+Shift+Space".to_string();
        assert!(complex_settings.validate().is_err());
    }

    #[test]
    fn test_keyboard_settings_sanitization() {
        let mut settings = KeyboardSettings::default();
        
        // Test empty shortcut sanitization
        settings.play_pause = "".to_string();
        let corrections = settings.sanitize();
        assert_eq!(settings.play_pause, "Space");
        assert_eq!(corrections.len(), 1);
        assert!(corrections[0].contains("Play/Pause"));
        assert!(corrections[0].contains("Space"));

        // Test multiple empty shortcuts
        let mut settings = KeyboardSettings::default();
        settings.play_pause = "".to_string();
        settings.return_to_start = "".to_string();
        let corrections = settings.sanitize();
        assert_eq!(corrections.len(), 2);
    }

    #[test]
    fn test_keyboard_settings_platform_defaults() {
        let settings = KeyboardSettings::default();
        
        // Test that application shortcuts use the correct modifier for the platform
        if cfg!(target_os = "macos") {
            assert!(settings.new_project.contains("Cmd+N"));
            assert!(settings.save_project.contains("Cmd+S"));
            assert!(settings.open_settings.contains("Cmd+,"));
        } else {
            assert!(settings.new_project.contains("Ctrl+N"));
            assert!(settings.save_project.contains("Ctrl+S"));
            assert!(settings.open_settings.contains("Ctrl+,"));
        }

        // Test that non-application shortcuts are platform-independent
        assert_eq!(settings.play_pause, "Space");
        assert_eq!(settings.return_to_start, "Enter");
        assert_eq!(settings.timeline_step_back, "Left");
    }

    #[test]
    fn test_app_settings_with_keyboard() {
        let settings = AppSettings::default();
        assert!(settings.validate().is_ok());
        
        // Test that keyboard settings are included in serialization
        let json = serde_json::to_string_pretty(&settings).unwrap();
        let deserialized: AppSettings = serde_json::from_str(&json).unwrap();
        
        assert!(deserialized.validate().is_ok());
        assert_eq!(settings.keyboard.play_pause, deserialized.keyboard.play_pause);
        assert_eq!(settings.keyboard.new_project, deserialized.keyboard.new_project);
    }

    #[test]
    fn test_keyboard_shortcut_parsing() {
        // Test simple key parsing
        let (key, modifiers) = KeyboardSettings::parse_shortcut("Space").unwrap();
        assert_eq!(key, egui::Key::Space);
        assert_eq!(modifiers, egui::Modifiers::default());

        // Test key with single modifier
        let (key, modifiers) = KeyboardSettings::parse_shortcut("Ctrl+S").unwrap();
        assert_eq!(key, egui::Key::S);
        assert!(modifiers.ctrl);
        assert!(!modifiers.alt && !modifiers.shift);

        // Test key with multiple modifiers
        let (key, modifiers) = KeyboardSettings::parse_shortcut("Ctrl+Shift+S").unwrap();
        assert_eq!(key, egui::Key::S);
        assert!(modifiers.ctrl && modifiers.shift);

        // Test platform-specific modifier
        let (key, modifiers) = KeyboardSettings::parse_shortcut("Cmd+N").unwrap();
        assert_eq!(key, egui::Key::N);
        assert!(modifiers.mac_cmd);

        // Test invalid shortcuts
        assert!(KeyboardSettings::parse_shortcut("").is_none());
        assert!(KeyboardSettings::parse_shortcut("InvalidKey").is_none());
        assert!(KeyboardSettings::parse_shortcut("Ctrl+InvalidKey").is_none());
    }

    #[test]
    fn test_keyboard_shortcut_matching() {
        // Test simple key matching
        let modifiers = egui::Modifiers::default();
        assert!(KeyboardSettings::matches_shortcut("Space", egui::Key::Space, &modifiers));
        assert!(!KeyboardSettings::matches_shortcut("Space", egui::Key::Enter, &modifiers));

        // Test modifier key matching
        let ctrl_modifiers = egui::Modifiers { ctrl: true, ..Default::default() };
        assert!(KeyboardSettings::matches_shortcut("Ctrl+S", egui::Key::S, &ctrl_modifiers));
        assert!(!KeyboardSettings::matches_shortcut("Ctrl+S", egui::Key::S, &modifiers));

        // Test multiple modifiers
        let ctrl_shift_modifiers = egui::Modifiers { ctrl: true, shift: true, ..Default::default() };
        assert!(KeyboardSettings::matches_shortcut("Ctrl+Shift+S", egui::Key::S, &ctrl_shift_modifiers));
        assert!(!KeyboardSettings::matches_shortcut("Ctrl+Shift+S", egui::Key::S, &ctrl_modifiers));
    }

    #[test]
    fn test_keyboard_integration_with_app_settings() {
        // Test that keyboard settings integrate properly with the main app settings
        let app_settings = AppSettings::default();
        
        // Validate that keyboard settings exist and are valid
        assert!(app_settings.keyboard.validate().is_ok());
        
        // Test that all default shortcuts are properly formatted
        let keyboard = &app_settings.keyboard;
        
        // Test transport shortcuts
        assert!(KeyboardSettings::parse_shortcut(&keyboard.play_pause).is_some());
        assert!(KeyboardSettings::parse_shortcut(&keyboard.return_to_start).is_some());
        assert!(KeyboardSettings::parse_shortcut(&keyboard.stop_escape).is_some());
        
        // Test timeline navigation shortcuts
        assert!(KeyboardSettings::parse_shortcut(&keyboard.timeline_step_back).is_some());
        assert!(KeyboardSettings::parse_shortcut(&keyboard.timeline_step_forward).is_some());
        assert!(KeyboardSettings::parse_shortcut(&keyboard.timeline_jump_back).is_some());
        assert!(KeyboardSettings::parse_shortcut(&keyboard.timeline_jump_forward).is_some());
        assert!(KeyboardSettings::parse_shortcut(&keyboard.timeline_start).is_some());
        assert!(KeyboardSettings::parse_shortcut(&keyboard.timeline_end).is_some());
        
        // Test pattern editing shortcuts
        assert!(KeyboardSettings::parse_shortcut(&keyboard.pattern_clear).is_some());
        assert!(KeyboardSettings::parse_shortcut(&keyboard.pattern_select_all).is_some());
        
        // Test application shortcuts
        assert!(KeyboardSettings::parse_shortcut(&keyboard.new_project).is_some());
        assert!(KeyboardSettings::parse_shortcut(&keyboard.open_project).is_some());
        assert!(KeyboardSettings::parse_shortcut(&keyboard.save_project).is_some());
        assert!(KeyboardSettings::parse_shortcut(&keyboard.save_project_as).is_some());
        assert!(KeyboardSettings::parse_shortcut(&keyboard.open_settings).is_some());
    }

    #[test]
    fn test_keyboard_shortcut_conflict_detection() {
        let mut keyboard = KeyboardSettings::default();
        
        // Create a conflict by setting two shortcuts to the same value
        keyboard.play_pause = "Space".to_string();
        keyboard.return_to_start = "Space".to_string(); // Conflict!
        
        // Validation should detect the conflict
        assert!(keyboard.validate().is_err());
        
        // Reset to defaults should pass validation
        keyboard = KeyboardSettings::default();
        assert!(keyboard.validate().is_ok());
    }

    #[test]
    fn test_keyboard_shortcut_cross_platform_functionality() {
        // Test that shortcuts work correctly across platforms
        let keyboard = KeyboardSettings::default();
        
        // Test platform-agnostic shortcuts (should work on all platforms)
        assert!(KeyboardSettings::matches_shortcut(&keyboard.play_pause, egui::Key::Space, &egui::Modifiers::default()));
        assert!(KeyboardSettings::matches_shortcut(&keyboard.return_to_start, egui::Key::Enter, &egui::Modifiers::default()));
        assert!(KeyboardSettings::matches_shortcut(&keyboard.timeline_step_back, egui::Key::ArrowLeft, &egui::Modifiers::default()));
        
        // Test platform-specific shortcuts
        if cfg!(target_os = "macos") {
            // On macOS, application shortcuts should use Cmd
            let cmd_modifiers = egui::Modifiers { mac_cmd: true, ..Default::default() };
            assert!(keyboard.new_project.contains("Cmd"));
            assert!(KeyboardSettings::matches_shortcut(&keyboard.new_project, egui::Key::N, &cmd_modifiers));
        } else {
            // On other platforms, application shortcuts should use Ctrl
            let ctrl_modifiers = egui::Modifiers { ctrl: true, ..Default::default() };
            assert!(keyboard.new_project.contains("Ctrl"));
            assert!(KeyboardSettings::matches_shortcut(&keyboard.new_project, egui::Key::N, &ctrl_modifiers));
        }
    }
}
