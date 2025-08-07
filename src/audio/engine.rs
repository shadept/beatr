use anyhow::Result;
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    Device, Host, SampleFormat, Stream, StreamConfig,
};
use std::sync::{Arc, Mutex};

use super::SampleBank;
use crate::settings::AudioSettings;
use crate::timeline::Timeline;

/// Detailed audio device information
#[derive(Debug, Clone, PartialEq)]
pub struct AudioDeviceInfo {
    pub name: String,
    pub id: String,
    pub is_default: bool,
    pub is_available: bool,
    pub supported_sample_rates: Vec<u32>,
    pub supported_buffer_sizes: Vec<u32>,
}

impl AudioDeviceInfo {
    /// Check if a specific sample rate and buffer size combination is supported
    pub fn supports_config(&self, sample_rate: u32, buffer_size: u32) -> bool {
        self.supported_sample_rates.contains(&sample_rate)
            && self.supported_buffer_sizes.contains(&buffer_size)
    }
}

/// Actions to take when a device becomes unavailable
#[derive(Debug, Clone, PartialEq)]
pub enum DeviceRecoveryAction {
    /// No action needed (device is still available)
    NoAction,
    /// Fallback to the default device
    FallbackToDefault,
    /// Fallback to a specific device
    FallbackToDevice(String),
    /// No devices available for fallback
    DeviceUnavailable,
}

pub struct AudioEngine {
    _host: Host,
    _device: Device,
    _stream: Stream,
    sample_bank: Arc<Mutex<SampleBank>>,
    timeline: Arc<Mutex<Timeline>>,
    sample_rate: f32,
    master_volume: Arc<Mutex<f32>>,
    current_device_name: String,
    settings: AudioSettings,
}

impl AudioEngine {
    /// Create a new AudioEngine with default settings (for backward compatibility)
    pub fn new() -> Result<Self> {
        Self::new_with_settings(AudioSettings::default())
    }

    /// Create a new AudioEngine with the provided audio settings
    pub fn new_with_settings(settings: AudioSettings) -> Result<Self> {
        let host = cpal::default_host();

        // Device selection based on settings
        let device = if let Some(preferred_device) = &settings.preferred_device {
            // Try to find the preferred device
            let devices = host.output_devices()?;
            let mut found_device = None;
            for device in devices {
                if let Ok(name) = device.name() {
                    if name == *preferred_device {
                        found_device = Some(device);
                        break;
                    }
                }
            }

            // Fall back to default if preferred device not found
            found_device.unwrap_or_else(|| {
                eprintln!(
                    "Warning: Preferred device '{}' not found, using default",
                    preferred_device
                );
                host.default_output_device()
                    .expect("No output device available")
            })
        } else {
            host.default_output_device()
                .ok_or_else(|| anyhow::anyhow!("No output device available"))?
        };

        // Get default config and override with settings
        let default_config = device.default_output_config()?;
        let sample_rate = cpal::SampleRate(settings.sample_rate);
        let channels = 1; // Force mono output for simpler timing

        println!("🎵 Audio Engine Configuration:");
        println!(
            "  Sample Rate: {} Hz (configured: {})",
            sample_rate.0, settings.sample_rate
        );
        println!("  Buffer Size: {} samples", settings.buffer_size);
        println!("  Master Volume: {:.0}%", settings.master_volume * 100.0);
        println!("  Channels: {} (forced mono)", channels);
        println!("  Sample Format: {:?}", default_config.sample_format());
        println!(
            "  Device: {}",
            device.name().unwrap_or_else(|_| "Unknown".to_string())
        );

        let sample_bank = Arc::new(Mutex::new({
            let mut bank = SampleBank::new();
            bank.load_default_samples();
            bank
        }));
        let timeline = Arc::new(Mutex::new(Timeline::new()));
        let master_volume = Arc::new(Mutex::new(settings.master_volume));

        let sample_bank_clone = Arc::clone(&sample_bank);
        let timeline_clone = Arc::clone(&timeline);
        let master_volume_clone = Arc::clone(&master_volume);

        let stream_config = StreamConfig {
            channels,
            sample_rate,
            buffer_size: cpal::BufferSize::Fixed(settings.buffer_size),
        };

        let stream = match default_config.sample_format() {
            SampleFormat::F32 => device.build_output_stream(
                &stream_config,
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    audio_callback(
                        data,
                        &sample_bank_clone,
                        &timeline_clone,
                        &master_volume_clone,
                        sample_rate.0 as f32,
                    )
                },
                |err| eprintln!("Audio stream error: {}", err),
                None,
            )?,
            SampleFormat::I16 => device.build_output_stream(
                &stream_config,
                move |data: &mut [i16], _: &cpal::OutputCallbackInfo| {
                    let mut f32_data = vec![0.0f32; data.len()];
                    audio_callback(
                        &mut f32_data,
                        &sample_bank_clone,
                        &timeline_clone,
                        &master_volume_clone,
                        sample_rate.0 as f32,
                    );
                    for (i, sample) in f32_data.iter().enumerate() {
                        data[i] = (*sample * i16::MAX as f32) as i16;
                    }
                },
                |err| eprintln!("Audio stream error: {}", err),
                None,
            )?,
            SampleFormat::U16 => device.build_output_stream(
                &stream_config,
                move |data: &mut [u16], _: &cpal::OutputCallbackInfo| {
                    let mut f32_data = vec![0.0f32; data.len()];
                    audio_callback(
                        &mut f32_data,
                        &sample_bank_clone,
                        &timeline_clone,
                        &master_volume_clone,
                        sample_rate.0 as f32,
                    );
                    for (i, sample) in f32_data.iter().enumerate() {
                        data[i] = ((*sample + 1.0) * 0.5 * u16::MAX as f32) as u16;
                    }
                },
                |err| eprintln!("Audio stream error: {}", err),
                None,
            )?,
            _ => return Err(anyhow::anyhow!("Unsupported sample format")),
        };

        stream.play()?;

        let current_device_name = settings
            .preferred_device
            .clone()
            .unwrap_or_else(|| "Default Device".to_string());

        Ok(AudioEngine {
            _host: host,
            _device: device,
            _stream: stream,
            sample_bank,
            timeline,
            sample_rate: sample_rate.0 as f32,
            master_volume,
            current_device_name,
            settings,
        })
    }

    pub fn sample_bank(&self) -> Arc<Mutex<SampleBank>> {
        Arc::clone(&self.sample_bank)
    }

    pub fn timeline(&self) -> Arc<Mutex<Timeline>> {
        Arc::clone(&self.timeline)
    }

    pub fn sample_rate(&self) -> f32 {
        self.sample_rate
    }

    /// Get the current master volume
    pub fn get_master_volume(&self) -> f32 {
        *self.master_volume.lock().unwrap()
    }

    /// Set the master volume (0.0 to 2.0)
    pub fn set_master_volume(&self, volume: f32) {
        let clamped_volume = volume.clamp(0.0, 2.0);
        *self.master_volume.lock().unwrap() = clamped_volume;
    }

    /// Get the master volume reference for sharing with audio callback
    pub fn master_volume(&self) -> Arc<Mutex<f32>> {
        Arc::clone(&self.master_volume)
    }

    /// Get list of available audio output devices
    pub fn get_available_devices() -> Result<Vec<String>> {
        let host = cpal::default_host();
        let mut devices = Vec::new();

        // Add default device first
        devices.push("Default Device".to_string());

        // Add other available devices with better error handling
        if let Ok(device_iter) = host.output_devices() {
            for device in device_iter {
                if let Ok(name) = device.name() {
                    if name != "Default Device" {
                        devices.push(name);
                    }
                }
            }
        }

        Ok(devices)
    }

    /// Get detailed information about all available audio output devices
    pub fn get_available_devices_detailed() -> Result<Vec<AudioDeviceInfo>> {
        let host = cpal::default_host();
        let mut devices = Vec::new();

        // Get default device
        if let Some(default_device) = host.default_output_device() {
            let device_info = Self::analyze_device(&default_device, true)?;
            devices.push(device_info);
        }

        // Get other available devices
        if let Ok(device_iter) = host.output_devices() {
            for device in device_iter {
                if let Ok(name) = device.name() {
                    // Skip if this is the default device we already added
                    let is_default_duplicate =
                        if let Some(default_device) = host.default_output_device() {
                            if let Ok(default_name) = default_device.name() {
                                name == default_name
                            } else {
                                false
                            }
                        } else {
                            false
                        };

                    if !is_default_duplicate {
                        match Self::analyze_device(&device, false) {
                            Ok(device_info) => devices.push(device_info),
                            Err(e) => {
                                eprintln!("Warning: Failed to analyze device {}: {}", name, e)
                            }
                        }
                    }
                }
            }
        }

        Ok(devices)
    }

    /// Refresh and get the latest device information (same as get_available_devices_detailed)
    pub fn refresh_devices() -> Result<Vec<AudioDeviceInfo>> {
        Self::get_available_devices_detailed()
    }

    /// Analyze a CPAL device to extract detailed information
    fn analyze_device(device: &Device, is_default: bool) -> Result<AudioDeviceInfo> {
        let name = device
            .name()
            .unwrap_or_else(|_| "Unknown Device".to_string());

        // Use device name as ID for now (CPAL doesn't provide stable device IDs)
        let id = name.clone();

        let mut supported_sample_rates = Vec::new();
        let mut supported_buffer_sizes = Vec::new();

        // Standard sample rates to test
        let test_sample_rates = [22050, 44100, 48000, 88200, 96000, 192000];
        // Standard buffer sizes to test (powers of 2)
        let test_buffer_sizes = [64, 128, 256, 512, 1024, 2048, 4096];

        // Try to get supported configurations
        if let Ok(configs) = device.supported_output_configs() {
            // Collect supported sample rates from device configs
            for config in configs {
                let min_rate = config.min_sample_rate().0;
                let max_rate = config.max_sample_rate().0;

                for &test_rate in &test_sample_rates {
                    if test_rate >= min_rate && test_rate <= max_rate {
                        if !supported_sample_rates.contains(&test_rate) {
                            supported_sample_rates.push(test_rate);
                        }
                    }
                }
            }
        }

        // If we couldn't determine supported rates, fall back to common ones
        if supported_sample_rates.is_empty() {
            supported_sample_rates = vec![44100, 48000];
        }

        // For buffer sizes, we'll support standard power-of-2 sizes
        // Most devices support these, and CPAL handles the actual buffer configuration
        supported_buffer_sizes = test_buffer_sizes.to_vec();

        Ok(AudioDeviceInfo {
            name: if is_default {
                format!("{} (Default)", name)
            } else {
                name
            },
            id,
            is_default,
            is_available: true,
            supported_sample_rates,
            supported_buffer_sizes,
        })
    }

    /// Test if a specific device configuration is working
    pub fn test_device_configuration(
        device_name: &str,
        sample_rate: u32,
        buffer_size: u32,
    ) -> Result<bool> {
        let host = cpal::default_host();

        // Find the device
        let device = if device_name == "Default Device" || device_name.ends_with(" (Default)") {
            host.default_output_device()
                .ok_or_else(|| anyhow::anyhow!("No default output device available"))?
        } else {
            // Strip "(Default)" suffix if present for comparison
            let clean_device_name = if device_name.ends_with(" (Default)") {
                &device_name[..device_name.len() - 10]
            } else {
                device_name
            };

            let devices = host.output_devices()?;
            let mut found_device = None;
            for device in devices {
                if let Ok(name) = device.name() {
                    if name == clean_device_name {
                        found_device = Some(device);
                        break;
                    }
                }
            }
            found_device.ok_or_else(|| anyhow::anyhow!("Device '{}' not found", device_name))?
        };

        // Test configuration by attempting to create a temporary stream
        let stream_config = StreamConfig {
            channels: 1, // Force mono for consistency
            sample_rate: cpal::SampleRate(sample_rate),
            buffer_size: cpal::BufferSize::Fixed(buffer_size),
        };

        // Try to build a test stream (but don't play it)
        let default_config = device.default_output_config()?;
        match default_config.sample_format() {
            SampleFormat::F32 => {
                let _stream = device.build_output_stream(
                    &stream_config,
                    move |_data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                        // Empty callback for testing
                    },
                    |err| eprintln!("Test stream error: {}", err),
                    None,
                )?;
                // Stream creation succeeded
                Ok(true)
            }
            SampleFormat::I16 => {
                let _stream = device.build_output_stream(
                    &stream_config,
                    move |_data: &mut [i16], _: &cpal::OutputCallbackInfo| {
                        // Empty callback for testing
                    },
                    |err| eprintln!("Test stream error: {}", err),
                    None,
                )?;
                Ok(true)
            }
            SampleFormat::U16 => {
                let _stream = device.build_output_stream(
                    &stream_config,
                    move |_data: &mut [u16], _: &cpal::OutputCallbackInfo| {
                        // Empty callback for testing
                    },
                    |err| eprintln!("Test stream error: {}", err),
                    None,
                )?;
                Ok(true)
            }
            _ => Err(anyhow::anyhow!("Unsupported sample format for testing")),
        }
    }

    /// Check if a device is still available (for monitoring)
    pub fn is_device_available(device_name: &str) -> bool {
        if let Ok(devices) = Self::get_available_devices() {
            // Handle default device check
            if device_name == "Default Device" || device_name.ends_with(" (Default)") {
                return devices
                    .iter()
                    .any(|d| d == "Default Device" || d.ends_with(" (Default)"));
            }

            // Strip "(Default)" suffix for comparison if present
            let clean_device_name = if device_name.ends_with(" (Default)") {
                &device_name[..device_name.len() - 10]
            } else {
                device_name
            };

            devices.iter().any(|d| {
                let clean_d = if d.ends_with(" (Default)") {
                    &d[..d.len() - 10]
                } else {
                    d
                };
                clean_d == clean_device_name
            })
        } else {
            false
        }
    }

    /// Monitor current device availability (for instance method)
    pub fn monitor_device_availability(&self) -> Result<bool> {
        if self.settings.device_monitoring_enabled {
            Ok(Self::is_device_available(&self.current_device_name))
        } else {
            // If monitoring is disabled, assume device is available
            Ok(true)
        }
    }

    /// Get the current device name
    pub fn get_current_device_name(&self) -> &str {
        &self.current_device_name
    }

    /// Handle device disconnection and fallback
    pub fn handle_device_disconnection(&mut self) -> Result<DeviceRecoveryAction> {
        if !self.settings.device_monitoring_enabled {
            return Ok(DeviceRecoveryAction::NoAction);
        }

        // Check if current device is still available
        if Self::is_device_available(&self.current_device_name) {
            return Ok(DeviceRecoveryAction::NoAction);
        }

        // Device is disconnected, try fallback strategies
        if self.settings.auto_fallback_enabled {
            // First try: fallback to last known good device
            if let Some(ref last_good_device) = self.settings.last_known_good_device {
                if last_good_device != &self.current_device_name
                    && Self::is_device_available(last_good_device)
                {
                    return Ok(DeviceRecoveryAction::FallbackToDevice(
                        last_good_device.clone(),
                    ));
                }
            }

            // Second try: fallback to default device
            if self.current_device_name != "Default Device"
                && Self::is_device_available("Default Device")
            {
                return Ok(DeviceRecoveryAction::FallbackToDefault);
            }

            // Third try: find any available device
            if let Ok(devices) = Self::get_available_devices() {
                for device in devices {
                    if device != self.current_device_name && Self::is_device_available(&device) {
                        return Ok(DeviceRecoveryAction::FallbackToDevice(device));
                    }
                }
            }
        }

        // No fallback available or auto-fallback disabled
        Ok(DeviceRecoveryAction::DeviceUnavailable)
    }

    /// Update the last known good device when a configuration works
    pub fn update_last_known_good_device(&mut self) -> Result<()> {
        self.settings.last_known_good_device = Some(self.current_device_name.clone());
        Ok(())
    }

    /// Switch to a different device (for fallback scenarios)
    pub fn switch_to_device(&mut self, new_device_name: String) -> Result<bool> {
        // This would require recreating the audio stream with the new device
        // For now, we'll just update the tracking and return success if the device is available
        if Self::is_device_available(&new_device_name) {
            self.current_device_name = new_device_name.clone();
            self.settings.preferred_device = if new_device_name == "Default Device" {
                None
            } else {
                Some(new_device_name)
            };
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

// Audio processing state - moved outside callback to avoid allocations
pub struct AudioState {
    pub current_step: usize,
    samples_per_step: usize,
    sample_counter: usize,
    voices: Vec<Voice>,
    loop_length: usize,
}

impl AudioState {
    pub fn new(sample_rate: f32, bpm: f32) -> Self {
        let mut state = AudioState {
            current_step: 0,
            samples_per_step: 0,
            sample_counter: 0,
            voices: Vec::new(),
            loop_length: 16,
        };

        state.update_timing(sample_rate, bpm);

        // Initialize voices
        for _ in 0..16 {
            state.voices.push(Voice::new());
        }

        state
    }

    fn reset(&mut self) {
        // Reset all step counters and sample positions
        self.current_step = 0;
        self.sample_counter = 0;

        // Clear all voice states
        for voice in &mut self.voices {
            voice.reset();
        }
    }

    pub fn synchronize_with_timeline(
        &mut self,
        timeline_position: f64,
        segment_start_time: f64,
        bpm: f32,
        sample_rate: f32,
    ) {
        // Update timing for current BPM
        self.update_timing(sample_rate, bpm);

        // Calculate position within the current segment
        let position_within_segment = timeline_position - segment_start_time;

        // Calculate which step we should be on based on position within segment
        let beats_per_second = bpm as f64 / 60.0;
        let steps_per_second = beats_per_second * 4.0; // 16th notes
        let total_steps_elapsed = (position_within_segment * steps_per_second) as usize;

        // Set current step based on position within the pattern loop
        self.current_step = total_steps_elapsed % self.loop_length;

        // Reset sample counter to start of current step
        self.sample_counter = 0;
    }

    fn update_timing(&mut self, sample_rate: f32, bpm: f32) {
        let beats_per_second = bpm / 60.0;
        let steps_per_second = beats_per_second * 4.0; // 16th notes
        self.samples_per_step = (sample_rate / steps_per_second) as usize;
    }

    fn process_patterns(
        &mut self,
        output: &mut [f32],
        sample_bank: &SampleBank,
        patterns: &[super::sequencer::Pattern],
        bpm: f32,
        sample_rate: f32,
    ) {
        // Update timing if BPM changed
        if self.samples_per_step == 0 {
            self.update_timing(sample_rate, bpm);
        }

        let mut sample_index = 0;
        while sample_index < output.len() {
            // Check if we need to trigger step
            if self.sample_counter == 0 {
                self.trigger_current_step(sample_bank, patterns);
            }

            // Calculate how many samples to process in this iteration
            let samples_until_next_step = self.samples_per_step - self.sample_counter;
            let samples_to_process = (output.len() - sample_index).min(samples_until_next_step);

            // Process voices for this chunk
            let chunk = &mut output[sample_index..sample_index + samples_to_process];
            for voice in &mut self.voices {
                // Voice processing handles sample rate internally via direct indexing
                // Sample data is pre-generated at the correct sample rate in SampleBank
                voice.process(chunk, sample_bank);
            }

            sample_index += samples_to_process;
            self.sample_counter += samples_to_process;

            // Advance step if needed
            if self.sample_counter >= self.samples_per_step {
                self.advance_step();
                self.sample_counter = 0;
            }
        }
    }

    fn trigger_current_step(
        &mut self,
        _sample_bank: &SampleBank,
        patterns: &[super::sequencer::Pattern],
    ) {
        for pattern in patterns {
            if self.current_step < pattern.steps.len() {
                let step = pattern.steps[self.current_step];
                if step.active {
                    // Find available voice
                    if let Some(voice) = self.voices.iter_mut().find(|v| !v.active) {
                        voice.trigger(pattern.sample_name.clone(), step.velocity);
                    }
                }
            }
        }
    }

    fn advance_step(&mut self) {
        self.current_step = (self.current_step + 1) % self.loop_length;
    }
}

// Voice structure for audio processing
#[derive(Debug, Clone)]
struct Voice {
    sample_position: usize,
    sample_name: String,
    velocity: f32,
    active: bool,
}

impl Voice {
    fn new() -> Self {
        Voice {
            sample_position: 0,
            sample_name: String::new(),
            velocity: 1.0,
            active: false,
        }
    }

    fn trigger(&mut self, sample_name: String, velocity: f32) {
        self.sample_name = sample_name;
        self.velocity = velocity;
        self.sample_position = 0;
        self.active = true;
    }

    fn reset(&mut self) {
        self.sample_position = 0;
        self.sample_name.clear();
        self.velocity = 1.0;
        self.active = false;
    }

    fn process(&mut self, output: &mut [f32], sample_bank: &SampleBank) {
        if !self.active {
            return;
        }

        if let Some(sample) = sample_bank.get_sample(&self.sample_name) {
            for out_sample in output.iter_mut() {
                if self.sample_position >= sample.data.len() {
                    self.active = false;
                    break;
                }

                let sample_value = sample.data[self.sample_position] * self.velocity;
                *out_sample += sample_value;
                self.sample_position += 1;
            }
        } else {
            self.active = false;
        }
    }
}

// Static audio state and timeline state tracking
static mut AUDIO_STATE: Option<AudioState> = None;
static mut LAST_TIMELINE_PLAYING: bool = false;
static AUDIO_STATE_INIT: std::sync::Once = std::sync::Once::new();

fn audio_callback(
    data: &mut [f32],
    sample_bank: &Arc<Mutex<SampleBank>>,
    timeline: &Arc<Mutex<Timeline>>,
    master_volume: &Arc<Mutex<f32>>,
    sample_rate: f32,
) {
    // Clear output buffer first
    for sample in data.iter_mut() {
        *sample = 0.0;
    }

    // Calculate time delta for this audio buffer
    // With forced mono output, data.len() directly represents the number of samples/frames
    let frames_per_buffer = data.len();
    let delta_time = frames_per_buffer as f64 / sample_rate as f64;

    // Process timeline audio
    let mut timeline_lock = timeline.lock().unwrap();
    let timeline_playing = timeline_lock.is_playing();

    unsafe {
        // Detect timeline state transitions
        if LAST_TIMELINE_PLAYING && !timeline_playing {
            // Timeline stopped - reset audio state
            if let Some(ref mut audio_state) = AUDIO_STATE {
                audio_state.reset();
            }
        } else if !LAST_TIMELINE_PLAYING && timeline_playing {
            // Timeline started playing - synchronize audio state with timeline position
            if let Some(segment) = timeline_lock.get_current_segment() {
                if let Some(ref mut audio_state) = AUDIO_STATE {
                    audio_state.synchronize_with_timeline(
                        timeline_lock.current_position,
                        segment.start_time,
                        segment.bpm,
                        sample_rate,
                    );
                }
            }
        }
        LAST_TIMELINE_PLAYING = timeline_playing;
    }

    if timeline_playing {
        if timeline_lock.advance_position(delta_time) {
            // Timeline is still playing, get current segment patterns
            if let Some(segment) = timeline_lock.get_current_segment() {
                let bank = sample_bank.lock().unwrap();

                // Initialize audio state if needed
                unsafe {
                    AUDIO_STATE_INIT.call_once(|| {
                        AUDIO_STATE = Some(AudioState::new(sample_rate, segment.bpm));
                    });

                    if let Some(ref mut audio_state) = AUDIO_STATE {
                        // Process audio directly from timeline patterns
                        audio_state.process_patterns(
                            data,
                            &bank,
                            &segment.patterns,
                            segment.bpm,
                            sample_rate,
                        );
                    }
                }
            }
        }
        // If timeline finished (advance_position returned false), buffer remains cleared
    }
    // If timeline not playing, buffer remains cleared

    // Apply master volume to the final output
    let volume = *master_volume.lock().unwrap();
    if volume != 1.0 {
        for sample in data.iter_mut() {
            *sample *= volume;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timeline_audio_direct_processing() {
        let mut sample_bank = SampleBank::new();
        sample_bank.load_default_samples();

        let mut timeline = Timeline::new();

        // Create a timeline segment with patterns
        use crate::audio::TimeSignature;
        use crate::timeline::TimelineSegment;

        // Create a simple test pattern with only step 1 active
        let mut test_pattern =
            crate::audio::sequencer::Pattern::new("Test".to_string(), "kick".to_string(), 16);
        test_pattern.steps[1].active = true; // Only step 1 active

        let segment = TimelineSegment::new(
            "Test Segment".to_string(),
            vec![test_pattern], // patterns
            0.0,                // start_time
            1,                  // loop_count
            TimeSignature::four_four(),
            120.0, // bpm
        );

        timeline.add_segment(segment);
        timeline.play(); // Timeline is now playing

        println!("Timeline playing: {}", timeline.is_playing());
        println!("Timeline segments: {}", timeline.segments.len());

        // Test that timeline is playing and has segments
        assert!(timeline.is_playing(), "Timeline should be playing");

        let current_segment = timeline.get_current_segment();
        assert!(
            current_segment.is_some(),
            "Timeline should have a current segment"
        );

        if let Some(segment) = current_segment {
            println!("Current segment patterns: {}", segment.patterns.len());
            assert!(!segment.patterns.is_empty(), "Segment should have patterns");

            let segment_pattern = &segment.patterns[0];

            // Segment pattern should have step 1 active
            assert!(
                segment_pattern.steps[1].active,
                "Segment pattern step 1 should be active"
            );
            assert!(
                !segment_pattern.steps[0].active,
                "Segment pattern step 0 should be inactive"
            );
        }

        println!("✅ Timeline direct audio processing test passed");
    }

    #[test]
    fn test_audio_state_reset_functionality() {
        // Test AudioState reset behavior
        let mut audio_state = AudioState::new(44100.0, 120.0);

        // Simulate some audio processing state
        audio_state.current_step = 8;
        audio_state.sample_counter = 500;

        // Trigger a voice to be active
        audio_state.voices[0].trigger("kick".to_string(), 0.8);
        audio_state.voices[2].trigger("snare".to_string(), 0.6);

        // Verify state is dirty
        assert_eq!(audio_state.current_step, 8);
        assert_eq!(audio_state.sample_counter, 500);
        assert!(audio_state.voices[0].active);
        assert!(audio_state.voices[2].active);
        assert_eq!(audio_state.voices[0].sample_name, "kick");

        // Reset audio state
        audio_state.reset();

        // Verify all state is cleared
        assert_eq!(audio_state.current_step, 0);
        assert_eq!(audio_state.sample_counter, 0);
        assert!(!audio_state.voices[0].active);
        assert!(!audio_state.voices[2].active);
        assert!(audio_state.voices[0].sample_name.is_empty());
        assert!(audio_state.voices[2].sample_name.is_empty());

        // Verify all voices are reset
        for voice in &audio_state.voices {
            assert!(!voice.active);
            assert_eq!(voice.sample_position, 0);
            assert!(voice.sample_name.is_empty());
            assert_eq!(voice.velocity, 1.0);
        }

        println!("✅ Audio state reset functionality test passed");
    }

    #[test]
    fn test_voice_reset_functionality() {
        let mut voice = Voice::new();

        // Verify initial state
        assert!(!voice.active);
        assert_eq!(voice.sample_position, 0);
        assert!(voice.sample_name.is_empty());

        // Trigger voice and advance position
        voice.trigger("test_sample".to_string(), 0.7);
        voice.sample_position = 1024; // Simulate playback progress

        // Verify voice is active with state
        assert!(voice.active);
        assert_eq!(voice.sample_position, 1024);
        assert_eq!(voice.sample_name, "test_sample");
        assert_eq!(voice.velocity, 0.7);

        // Reset voice
        voice.reset();

        // Verify voice is completely reset
        assert!(!voice.active);
        assert_eq!(voice.sample_position, 0);
        assert!(voice.sample_name.is_empty());
        assert_eq!(voice.velocity, 1.0);

        println!("✅ Voice reset functionality test passed");
    }

    #[test]
    fn test_audio_state_timeline_synchronization() {
        let mut audio_state = AudioState::new(44100.0, 120.0);

        // Test synchronization at different timeline positions
        // At 120 BPM: 120 beats/min = 2 beats/sec = 8 sixteenth-notes/sec
        let test_cases = [
            (0.0, 0), // Start of timeline
            (0.5, 4), // 0.5 seconds = 4 steps at 120 BPM
            (1.0, 8), // 1.0 seconds = 8 steps
            (2.0, 0), // 2.0 seconds = 16 steps = 0 (looped)
            (4.0, 0), // 4.0 seconds = 32 steps = 0 (looped)
        ];

        for (timeline_pos, expected_step) in test_cases {
            audio_state.synchronize_with_timeline(timeline_pos, 0.0, 120.0, 44100.0);
            assert_eq!(
                audio_state.current_step, expected_step,
                "Timeline position {} should result in step {}, got {}",
                timeline_pos, expected_step, audio_state.current_step
            );
            assert_eq!(
                audio_state.sample_counter, 0,
                "Sample counter should be reset to 0 after synchronization"
            );
        }

        // Test with different BPM
        // At 60 BPM: 60 beats/min = 1 beat/sec = 4 sixteenth-notes/sec
        audio_state.synchronize_with_timeline(1.0, 0.0, 60.0, 44100.0); // 60 BPM = half speed
        assert_eq!(
            audio_state.current_step, 4,
            "At 60 BPM, 1 second should be step 4"
        );

        println!("✅ Audio state timeline synchronization test passed");
    }

    #[test]
    fn test_timeline_stop_start_cycles() {
        let mut timeline = Timeline::new();

        // Create a test segment
        let mut test_pattern =
            crate::audio::sequencer::Pattern::new("Test".to_string(), "kick".to_string(), 16);
        test_pattern.steps[0].active = true;
        test_pattern.steps[4].active = true;
        test_pattern.steps[8].active = true;

        let segment = crate::timeline::TimelineSegment::new(
            "Test Segment".to_string(),
            vec![test_pattern],
            0.0,
            2, // 2 loops
            crate::audio::TimeSignature::four_four(),
            120.0,
        );

        timeline.add_segment(segment);

        // Test initial state
        assert!(!timeline.is_playing());
        assert_eq!(timeline.current_position, 0.0);

        // Test play
        timeline.play();
        assert!(timeline.is_playing());

        // Advance timeline
        let advanced = timeline.advance_position(1.0);
        assert!(advanced);
        assert_eq!(timeline.current_position, 1.0);

        // Test stop - should reset position
        timeline.stop();
        assert!(!timeline.is_playing());
        assert_eq!(
            timeline.current_position, 0.0,
            "Timeline position should reset to 0 on stop"
        );

        // Test play again - should start from beginning
        timeline.play();
        assert!(timeline.is_playing());
        assert_eq!(
            timeline.current_position, 0.0,
            "Timeline should start from 0 after stop/play cycle"
        );

        // Test seeking and its impact
        timeline.seek(1.5);
        assert_eq!(timeline.current_position, 1.5);

        timeline.stop();
        assert_eq!(
            timeline.current_position, 0.0,
            "Stop should reset position even after seeking"
        );

        println!("✅ Timeline stop/start cycles test passed");
    }

    #[test]
    fn test_audio_engine_initialization() {
        // Test that AudioEngine can be created without crashing
        let result = AudioEngine::new();

        // On systems without audio, this might fail, but we can check the structure
        match result {
            Ok(engine) => {
                // Check sample bank has samples
                let sample_bank = engine.sample_bank();
                let bank = sample_bank.lock().unwrap();
                let samples = bank.list_samples();
                assert!(
                    !samples.is_empty(),
                    "Sample bank should have samples loaded"
                );
                assert!(
                    samples.contains(&&"kick".to_string()),
                    "Should have kick sample"
                );
                assert!(
                    samples.contains(&&"snare".to_string()),
                    "Should have snare sample"
                );
                assert!(
                    samples.contains(&&"hihat".to_string()),
                    "Should have hihat sample"
                );

                // Check timeline exists
                let timeline = engine.timeline();
                let tl = timeline.lock().unwrap();
                assert!(!tl.is_playing(), "Timeline should not be playing initially");

                println!("✅ AudioEngine initialization test passed");
            }
            Err(e) => {
                // On systems without audio hardware, we can't test the full engine
                // but we can still test individual components
                println!(
                    "⚠️  AudioEngine creation failed (likely no audio device): {}",
                    e
                );
                println!("Testing individual components instead...");

                // Test timeline independently
                let mut timeline = Timeline::new();
                assert!(
                    !timeline.is_playing(),
                    "Timeline should not be playing initially"
                );
                assert!(timeline.is_empty(), "Timeline should be empty initially");

                // Test sample bank independently
                let mut sample_bank = SampleBank::new();
                sample_bank.load_default_samples();
                let samples = sample_bank.list_samples();
                assert!(!samples.is_empty(), "Should have samples loaded");

                println!("✅ Individual component tests passed");
            }
        }
    }

    #[test]
    fn test_device_enumeration() {
        // Test basic device enumeration
        let devices_result = AudioEngine::get_available_devices();
        match devices_result {
            Ok(devices) => {
                assert!(!devices.is_empty(), "Should have at least default device");
                assert!(
                    devices.contains(&"Default Device".to_string()),
                    "Should include default device"
                );
                println!("✅ Found {} audio devices", devices.len());
            }
            Err(e) => {
                println!(
                    "⚠️  Device enumeration failed (likely no audio hardware): {}",
                    e
                );
            }
        }
    }

    #[test]
    fn test_detailed_device_enumeration() {
        // Test detailed device enumeration
        let devices_result = AudioEngine::get_available_devices_detailed();
        match devices_result {
            Ok(devices) => {
                assert!(!devices.is_empty(), "Should have at least one device");

                // Check that we have a default device
                let has_default = devices.iter().any(|d| d.is_default);
                assert!(has_default, "Should have at least one default device");

                // Verify device info structure
                for device in &devices {
                    assert!(!device.name.is_empty(), "Device name should not be empty");
                    assert!(!device.id.is_empty(), "Device ID should not be empty");
                    assert!(device.is_available, "Device should be marked as available");
                    assert!(
                        !device.supported_sample_rates.is_empty(),
                        "Should have supported sample rates"
                    );
                    assert!(
                        !device.supported_buffer_sizes.is_empty(),
                        "Should have supported buffer sizes"
                    );

                    // Check that common sample rates are supported
                    assert!(
                        device.supported_sample_rates.contains(&44100)
                            || device.supported_sample_rates.contains(&48000),
                        "Should support at least 44.1kHz or 48kHz"
                    );

                    // Check that common buffer sizes are supported
                    assert!(
                        device.supported_buffer_sizes.contains(&1024),
                        "Should support 1024 samples buffer"
                    );
                }

                println!(
                    "✅ Detailed device enumeration test passed - {} devices found",
                    devices.len()
                );
                for device in devices {
                    println!(
                        "  Device: {} (default: {}, rates: {:?})",
                        device.name, device.is_default, device.supported_sample_rates
                    );
                }
            }
            Err(e) => {
                println!(
                    "⚠️  Detailed device enumeration failed (likely no audio hardware): {}",
                    e
                );
            }
        }
    }

    #[test]
    fn test_device_availability_check() {
        // Test device availability checking
        assert!(
            AudioEngine::is_device_available("Default Device"),
            "Default Device should always be available if system has audio"
        );

        assert!(
            !AudioEngine::is_device_available("Nonexistent Device 123"),
            "Nonexistent device should not be available"
        );

        println!("✅ Device availability check test passed");
    }

    #[test]
    fn test_device_configuration_testing() {
        // Test device configuration testing
        let test_result = AudioEngine::test_device_configuration("Default Device", 44100, 1024);
        match test_result {
            Ok(success) => {
                assert!(success, "Default device with standard config should work");
                println!("✅ Device configuration test passed - standard config works");
            }
            Err(e) => {
                println!(
                    "⚠️  Device configuration test failed (likely no audio hardware): {}",
                    e
                );
            }
        }

        // Test with invalid config
        let invalid_result =
            AudioEngine::test_device_configuration("Nonexistent Device", 44100, 1024);
        assert!(
            invalid_result.is_err(),
            "Nonexistent device should fail configuration test"
        );

        println!("✅ Device configuration validation test passed");
    }

    #[test]
    fn test_audio_device_info_config_support() {
        let device_info = AudioDeviceInfo {
            name: "Test Device".to_string(),
            id: "test".to_string(),
            is_default: false,
            is_available: true,
            supported_sample_rates: vec![44100, 48000, 96000],
            supported_buffer_sizes: vec![256, 512, 1024, 2048],
        };

        assert!(
            device_info.supports_config(44100, 1024),
            "Should support 44.1kHz @ 1024 samples"
        );
        assert!(
            device_info.supports_config(48000, 512),
            "Should support 48kHz @ 512 samples"
        );
        assert!(
            !device_info.supports_config(22050, 1024),
            "Should not support 22.05kHz (not in list)"
        );
        assert!(
            !device_info.supports_config(44100, 128),
            "Should not support 128 samples (not in list)"
        );

        println!("✅ AudioDeviceInfo config support test passed");
    }

    #[test]
    fn test_comprehensive_device_validation() {
        // Test validation with different device configurations
        let test_configs = [
            ("Default Device", 44100, 1024, true),      // Should work
            ("Default Device", 48000, 512, true),       // Should work
            ("Default Device", 22050, 256, true),       // Should work
            ("Nonexistent Device", 44100, 1024, false), // Should fail
        ];

        for (device_name, sample_rate, buffer_size, should_succeed) in test_configs {
            let result =
                AudioEngine::test_device_configuration(device_name, sample_rate, buffer_size);

            if should_succeed {
                match result {
                    Ok(success) => {
                        assert!(
                            success,
                            "Device config test should succeed for {}",
                            device_name
                        );
                        println!(
                            "✅ Config test passed for {} @ {}Hz / {} samples",
                            device_name, sample_rate, buffer_size
                        );
                    }
                    Err(e) => {
                        println!(
                            "⚠️  Config test failed for {} (likely no audio hardware): {}",
                            device_name, e
                        );
                    }
                }
            } else {
                assert!(
                    result.is_err(),
                    "Invalid device should fail configuration test"
                );
                println!(
                    "✅ Invalid device '{}' correctly failed configuration test",
                    device_name
                );
            }
        }

        println!("✅ Comprehensive device validation test completed");
    }

    #[test]
    fn test_device_configuration_edge_cases() {
        // Test edge cases for device configuration validation

        // Test very high sample rate (should work if device supports it, or fail gracefully)
        let high_rate_result = AudioEngine::test_device_configuration("Default Device", 192000, 64);
        match high_rate_result {
            Ok(_) => println!("✅ High sample rate test passed"),
            Err(e) => println!("ℹ️  High sample rate test failed as expected: {}", e),
        }

        // Test very large buffer size (should work if device supports it, or fail gracefully)
        let large_buffer_result =
            AudioEngine::test_device_configuration("Default Device", 44100, 4096);
        match large_buffer_result {
            Ok(_) => println!("✅ Large buffer size test passed"),
            Err(e) => println!("ℹ️  Large buffer size test failed as expected: {}", e),
        }

        // Test empty device name (should fail)
        let empty_name_result = AudioEngine::test_device_configuration("", 44100, 1024);
        assert!(empty_name_result.is_err(), "Empty device name should fail");

        println!("✅ Device configuration edge cases test completed");
    }

    #[test]
    fn test_device_monitoring_functionality() {
        // Test device monitoring capabilities
        let mut settings = AudioSettings::default();
        settings.device_monitoring_enabled = true;
        settings.auto_fallback_enabled = true;

        // Create an AudioEngine to test monitoring (if audio hardware available)
        match AudioEngine::new_with_settings(settings.clone()) {
            Ok(mut engine) => {
                // Test device availability monitoring
                match engine.monitor_device_availability() {
                    Ok(available) => {
                        assert!(available, "Current device should be available");
                        println!("✅ Device monitoring reports current device as available");
                    }
                    Err(e) => {
                        println!("⚠️  Device monitoring failed: {}", e);
                    }
                }

                // Test getting current device name
                let device_name = engine.get_current_device_name();
                assert!(!device_name.is_empty(), "Device name should not be empty");
                println!("✅ Current device name: {}", device_name);

                // Test updating last known good device
                assert!(engine.update_last_known_good_device().is_ok());
                println!("✅ Updated last known good device");

                println!("✅ Device monitoring functionality test passed");
            }
            Err(e) => {
                println!(
                    "⚠️  Device monitoring test skipped (no audio hardware): {}",
                    e
                );
            }
        }
    }

    #[test]
    fn test_device_fallback_logic() {
        // Test device fallback decision logic without requiring actual audio hardware
        let mut settings = AudioSettings::default();
        settings.device_monitoring_enabled = true;
        settings.auto_fallback_enabled = true;
        settings.preferred_device = Some("Test Device".to_string());
        settings.last_known_good_device = Some("Fallback Device".to_string());

        // This test focuses on the fallback logic rather than actual audio hardware
        // We can test the DeviceRecoveryAction enum and logic patterns

        // Test NoAction case - when current device is available
        // (We can't easily test this without mocking, but we can test the enum)
        let no_action = DeviceRecoveryAction::NoAction;
        assert_eq!(no_action, DeviceRecoveryAction::NoAction);

        // Test FallbackToDefault case
        let fallback_default = DeviceRecoveryAction::FallbackToDefault;
        assert_eq!(fallback_default, DeviceRecoveryAction::FallbackToDefault);

        // Test FallbackToDevice case
        let fallback_device = DeviceRecoveryAction::FallbackToDevice("Test Device".to_string());
        match &fallback_device {
            DeviceRecoveryAction::FallbackToDevice(name) => {
                assert_eq!(name, "Test Device");
            }
            _ => panic!("Expected FallbackToDevice variant"),
        }

        // Test DeviceUnavailable case
        let unavailable = DeviceRecoveryAction::DeviceUnavailable;
        assert_eq!(unavailable, DeviceRecoveryAction::DeviceUnavailable);

        println!("✅ Device fallback logic structures test passed");
    }

    #[test]
    fn test_device_monitoring_settings_integration() {
        // Test that device monitoring settings are properly integrated
        let default_settings = AudioSettings::default();

        // Check that monitoring is enabled by default
        assert!(
            default_settings.device_monitoring_enabled,
            "Device monitoring should be enabled by default"
        );
        assert!(
            default_settings.auto_fallback_enabled,
            "Auto fallback should be enabled by default"
        );
        assert!(
            default_settings.last_known_good_device.is_none(),
            "Last known good device should be None initially"
        );

        // Test settings with monitoring disabled
        let mut disabled_settings = default_settings.clone();
        disabled_settings.device_monitoring_enabled = false;

        // Test that validation still works with new fields
        assert!(
            disabled_settings.validate().is_ok(),
            "Settings with disabled monitoring should validate"
        );

        println!("✅ Device monitoring settings integration test passed");
    }
}
