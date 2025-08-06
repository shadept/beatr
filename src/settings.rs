use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use anyhow::Result;

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
            return Err(anyhow::anyhow!("Sample rate must be between 22050 and 192000 Hz"));
        }

        if ![64, 128, 256, 512, 1024, 2048, 4096].contains(&self.buffer_size) {
            return Err(anyhow::anyhow!("Buffer size must be a power of 2 between 64 and 4096"));
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
        let default_scale = if cfg!(target_os = "macos") {
            2.0
        } else {
            1.0
        };
        
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
            return Err(anyhow::anyhow!("Theme must be 'dark', 'light', or 'auto', got '{}'", self.theme));
        }

        // Check for NaN or infinite values first (before range checks)
        if !self.ui_scale.is_finite() {
            return Err(anyhow::anyhow!("UI scale must be a finite number, got {}", self.ui_scale));
        }
        
        // Validate scale range with more detailed error information
        if self.ui_scale < 0.5 {
            return Err(anyhow::anyhow!("UI scale {} is too small (minimum: 0.5)", self.ui_scale));
        }
        if self.ui_scale > 3.0 {
            return Err(anyhow::anyhow!("UI scale {} is too large (maximum: 3.0)", self.ui_scale));
        }

        Ok(())
    }

    /// Sanitize individual UI settings values, correcting invalid ones
    pub fn sanitize(&mut self) -> Vec<String> {
        let mut corrections = Vec::new();

        // Sanitize theme
        if !["dark", "light", "auto"].contains(&self.theme.as_str()) {
            corrections.push(format!("Theme '{}' is invalid, changed to 'dark'", self.theme));
            self.theme = "dark".to_string();
        }

        // Sanitize scale (check finite first, then range)
        if !self.ui_scale.is_finite() {
            let default_scale = UISettings::default().ui_scale;
            corrections.push(format!("UI scale {} is invalid, changed to {}", self.ui_scale, default_scale));
            self.ui_scale = default_scale;
        } else if self.ui_scale < 0.5 {
            corrections.push(format!("UI scale {} is too small, changed to 0.5", self.ui_scale));
            self.ui_scale = 0.5;
        } else if self.ui_scale > 3.0 {
            corrections.push(format!("UI scale {} is too large, changed to 3.0", self.ui_scale));
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
    /// Validate default settings values
    pub fn validate(&self) -> Result<()> {
        if self.default_bpm < 60.0 || self.default_bpm > 300.0 {
            return Err(anyhow::anyhow!("Default BPM must be between 60 and 300"));
        }

        let (numerator, denominator) = self.default_time_signature;
        if numerator < 1 || numerator > 16 {
            return Err(anyhow::anyhow!("Time signature numerator must be between 1 and 16"));
        }

        if ![1, 2, 4, 8, 16].contains(&denominator) {
            return Err(anyhow::anyhow!("Time signature denominator must be 1, 2, 4, 8, or 16"));
        }

        if self.default_pattern_length < 4 || self.default_pattern_length > 64 {
            return Err(anyhow::anyhow!("Default pattern length must be between 4 and 64 steps"));
        }

        Ok(())
    }
}

/// Main application settings structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub audio: AudioSettings,
    pub ui: UISettings,
    pub defaults: DefaultSettings,
}

impl Default for AppSettings {
    fn default() -> Self {
        AppSettings {
            audio: AudioSettings::default(),
            ui: UISettings::default(),
            defaults: DefaultSettings::default(),
        }
    }
}

impl AppSettings {

    /// Validate all settings
    pub fn validate(&self) -> Result<()> {
        self.audio.validate()?;
        self.ui.validate()?;
        self.defaults.validate()?;
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
                                    },
                                    Err(err) => {
                                        eprintln!("Warning: Failed to parse settings file: {}. Using defaults.", err);
                                        Self::default()
                                    }
                                }
                            },
                            Err(err) => {
                                eprintln!("Warning: Failed to read settings file: {}. Using defaults.", err);
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
                },
                Err(err) => {
                    eprintln!("Warning: Failed to determine settings file path: {}. Using defaults.", err);
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

        // Sanitize default settings (use coarse-grained approach for now)
        if settings.defaults.validate().is_err() {
            all_corrections.push("Default settings were invalid and reset to defaults".to_string());
            settings.defaults = DefaultSettings::default();
        }

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
        assert_eq!(original_settings.audio.sample_rate, deserialized_settings.audio.sample_rate);
        assert_eq!(original_settings.ui.theme, deserialized_settings.ui.theme);
        assert_eq!(original_settings.defaults.default_bpm, deserialized_settings.defaults.default_bpm);
    }

    #[test]
    fn test_settings_sanitization() {
        let mut invalid_settings = AppSettings::default();
        invalid_settings.audio.sample_rate = 10000; // Invalid
        invalid_settings.ui.theme = "invalid".to_string(); // Invalid
        invalid_settings.defaults.default_bpm = 500.0; // Invalid

        let sanitized = AppSettings::sanitize_settings(invalid_settings);
        assert!(sanitized.validate().is_ok());
        assert_eq!(sanitized.audio.sample_rate, AudioSettings::default().sample_rate);
        assert_eq!(sanitized.ui.theme, UISettings::default().theme);
        assert_eq!(sanitized.defaults.default_bpm, DefaultSettings::default().default_bpm);
    }

    #[test]
    fn test_auto_save_validation() {
        let mut settings = AppSettings::default();
        
        // Test auto-save with valid settings
        assert!(settings.auto_save().is_ok());
        
        // Test auto-save with invalid settings
        settings.audio.sample_rate = 10000; // Invalid
        assert!(settings.auto_save().is_err(), "Auto-save should fail with invalid settings");
        
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
}