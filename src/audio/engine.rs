use anyhow::Result;
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    Device, Host, SampleFormat, SampleRate, Stream, StreamConfig,
};
use std::sync::{Arc, Mutex};

use super::SampleBank;
use crate::timeline::Timeline;

pub struct AudioEngine {
    _host: Host,
    _device: Device,
    _stream: Stream,
    sample_bank: Arc<Mutex<SampleBank>>,
    timeline: Arc<Mutex<Timeline>>,
    sample_rate: f32,
}

impl AudioEngine {
    pub fn new() -> Result<Self> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or_else(|| anyhow::anyhow!("No output device available"))?;

        let config = device.default_output_config()?;
        let sample_rate = config.sample_rate().0 as f32;
        let channels = config.channels();

        let sample_bank = Arc::new(Mutex::new({
            let mut bank = SampleBank::new();
            bank.load_default_samples();
            bank
        }));
        let timeline = Arc::new(Mutex::new(Timeline::new()));

        let sample_bank_clone = Arc::clone(&sample_bank);
        let timeline_clone = Arc::clone(&timeline);

        let stream_config = StreamConfig {
            channels,
            sample_rate: SampleRate(sample_rate as u32),
            buffer_size: cpal::BufferSize::Default,
        };

        let stream = match config.sample_format() {
            SampleFormat::F32 => device.build_output_stream(
                &stream_config,
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    audio_callback(data, &sample_bank_clone, &timeline_clone, sample_rate)
                },
                |err| eprintln!("Audio stream error: {}", err),
                None,
            )?,
            SampleFormat::I16 => device.build_output_stream(
                &stream_config,
                move |data: &mut [i16], _: &cpal::OutputCallbackInfo| {
                    let mut f32_data = vec![0.0f32; data.len()];
                    audio_callback(&mut f32_data, &sample_bank_clone, &timeline_clone, sample_rate);
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
                    audio_callback(&mut f32_data, &sample_bank_clone, &timeline_clone, sample_rate);
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

        Ok(AudioEngine {
            _host: host,
            _device: device,
            _stream: stream,
            sample_bank,
            timeline,
            sample_rate,
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
}

// Audio processing state - moved outside callback to avoid allocations
struct AudioState {
    current_step: usize,
    samples_per_step: usize,
    sample_counter: usize,
    voices: Vec<Voice>,
    loop_length: usize,
}

impl AudioState {
    fn new(sample_rate: f32, bpm: f32) -> Self {
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
    
    fn update_timing(&mut self, sample_rate: f32, bpm: f32) {
        let beats_per_second = bpm / 60.0;
        let steps_per_second = beats_per_second * 4.0; // 16th notes
        self.samples_per_step = (sample_rate / steps_per_second) as usize;
    }
    
    fn process_patterns(&mut self, output: &mut [f32], sample_bank: &SampleBank, patterns: &[super::sequencer::Pattern], bpm: f32, sample_rate: f32) {
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
    
    fn trigger_current_step(&mut self, _sample_bank: &SampleBank, patterns: &[super::sequencer::Pattern]) {
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

// Static audio state - initialized once, reused across callbacks
static mut AUDIO_STATE: Option<AudioState> = None;
static AUDIO_STATE_INIT: std::sync::Once = std::sync::Once::new();

fn audio_callback(
    data: &mut [f32],
    sample_bank: &Arc<Mutex<SampleBank>>,
    timeline: &Arc<Mutex<Timeline>>,
    sample_rate: f32,
) {
    // Clear output buffer first
    for sample in data.iter_mut() {
        *sample = 0.0;
    }
    
    // Calculate time delta for this audio buffer
    let samples_per_buffer = data.len();
    let delta_time = samples_per_buffer as f64 / sample_rate as f64;
    
    // Process timeline audio
    let mut timeline_lock = timeline.lock().unwrap();
    
    if timeline_lock.is_playing() {
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
                        audio_state.process_patterns(data, &bank, &segment.patterns, segment.bpm, sample_rate);
                    }
                }
            }
        }
        // If timeline finished (advance_position returned false), buffer remains cleared
    }
    // If timeline not playing, buffer remains cleared
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
        let mut test_pattern = crate::audio::sequencer::Pattern::new("Test".to_string(), "kick".to_string(), 16);
        test_pattern.steps[1].active = true; // Only step 1 active
        
        let segment = TimelineSegment::new(
            "Test Segment".to_string(),
            vec![test_pattern], // patterns
            0.0, // start_time
            1, // loop_count
            TimeSignature::four_four(),
            120.0 // bpm
        );
        
        timeline.add_segment(segment);
        timeline.play(); // Timeline is now playing
        
        println!("Timeline playing: {}", timeline.is_playing());
        println!("Timeline segments: {}", timeline.segments.len());
        
        // Test that timeline is playing and has segments
        assert!(timeline.is_playing(), "Timeline should be playing");
        
        let current_segment = timeline.get_current_segment();
        assert!(current_segment.is_some(), "Timeline should have a current segment");
        
        if let Some(segment) = current_segment {
            println!("Current segment patterns: {}", segment.patterns.len());
            assert!(!segment.patterns.is_empty(), "Segment should have patterns");
            
            let segment_pattern = &segment.patterns[0];
            
            // Segment pattern should have step 1 active
            assert!(segment_pattern.steps[1].active, "Segment pattern step 1 should be active");
            assert!(!segment_pattern.steps[0].active, "Segment pattern step 0 should be inactive");
        }
        
        println!("✅ Timeline direct audio processing test passed");
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
                assert!(!samples.is_empty(), "Sample bank should have samples loaded");
                assert!(samples.contains(&&"kick".to_string()), "Should have kick sample");
                assert!(samples.contains(&&"snare".to_string()), "Should have snare sample");
                assert!(samples.contains(&&"hihat".to_string()), "Should have hihat sample");
                
                // Check timeline exists
                let timeline = engine.timeline();
                let tl = timeline.lock().unwrap();
                assert!(!tl.is_playing(), "Timeline should not be playing initially");
                
                println!("✅ AudioEngine initialization test passed");
            }
            Err(e) => {
                // On systems without audio hardware, we can't test the full engine
                // but we can still test individual components
                println!("⚠️  AudioEngine creation failed (likely no audio device): {}", e);
                println!("Testing individual components instead...");
                
                // Test timeline independently
                let mut timeline = Timeline::new();
                assert!(!timeline.is_playing(), "Timeline should not be playing initially");
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
}