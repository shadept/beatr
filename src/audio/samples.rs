use anyhow::Result;
use hound::WavReader;
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct Sample {
    pub data: Vec<f32>,
    pub sample_rate: u32,
    pub channels: u16,
}

impl Sample {
    pub fn from_wav_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let mut reader = WavReader::open(path)?;
        let spec = reader.spec();

        let samples: Result<Vec<f32>, _> = match spec.sample_format {
            hound::SampleFormat::Float => reader.samples::<f32>().collect(),
            hound::SampleFormat::Int => reader
                .samples::<i32>()
                .map(|s| s.map(|sample| sample as f32 / i32::MAX as f32))
                .collect(),
        };

        Ok(Sample {
            data: samples?,
            sample_rate: spec.sample_rate,
            channels: spec.channels,
        })
    }

    pub fn from_data(data: Vec<f32>, sample_rate: u32, channels: u16) -> Self {
        Sample {
            data,
            sample_rate,
            channels,
        }
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn duration_seconds(&self) -> f32 {
        self.len() as f32 / (self.sample_rate as f32 * self.channels as f32)
    }
}

#[derive(Debug)]
pub struct SampleBank {
    samples: HashMap<String, Sample>,
}

impl SampleBank {
    pub fn new() -> Self {
        SampleBank {
            samples: HashMap::new(),
        }
    }

    pub fn add_sample(&mut self, name: String, sample: Sample) {
        self.samples.insert(name, sample);
    }

    pub fn get_sample(&self, name: &str) -> Option<&Sample> {
        self.samples.get(name)
    }

    pub fn remove_sample(&mut self, name: &str) -> Option<Sample> {
        self.samples.remove(name)
    }

    pub fn list_samples(&self) -> Vec<&String> {
        self.samples.keys().collect()
    }

    pub fn load_default_samples(&mut self) {
        // Create some basic synthesized drum samples
        let sample_rate = 44100;
        
        // Kick drum - sine wave with exponential decay
        let kick_data = generate_kick(sample_rate as f32, 0.5);
        self.add_sample("kick".to_string(), Sample::from_data(kick_data, sample_rate, 1));

        // Snare - noise burst with envelope
        let snare_data = generate_snare(sample_rate as f32, 0.3);
        self.add_sample("snare".to_string(), Sample::from_data(snare_data, sample_rate, 1));

        // Hi-hat - filtered noise
        let hihat_data = generate_hihat(sample_rate as f32, 0.1);
        self.add_sample("hihat".to_string(), Sample::from_data(hihat_data, sample_rate, 1));

        // Crash cymbal - metallic noise with long decay
        let crash_data = generate_crash(sample_rate as f32, 2.0);
        self.add_sample("crash".to_string(), Sample::from_data(crash_data, sample_rate, 1));

        // Open hi-hat - filtered noise with medium decay
        let open_hihat_data = generate_open_hihat(sample_rate as f32, 0.4);
        self.add_sample("open_hihat".to_string(), Sample::from_data(open_hihat_data, sample_rate, 1));

        // Clap - burst of noise with rhythmic envelope
        let clap_data = generate_clap(sample_rate as f32, 0.2);
        self.add_sample("clap".to_string(), Sample::from_data(clap_data, sample_rate, 1));

        // Rim shot - sharp attack with quick decay
        let rimshot_data = generate_rimshot(sample_rate as f32, 0.1);
        self.add_sample("rimshot".to_string(), Sample::from_data(rimshot_data, sample_rate, 1));

        // Tom - pitched drum with decay
        let tom_data = generate_tom(sample_rate as f32, 0.6);
        self.add_sample("tom".to_string(), Sample::from_data(tom_data, sample_rate, 1));
    }
}

fn generate_kick(sample_rate: f32, duration: f32) -> Vec<f32> {
    let samples = (sample_rate * duration) as usize;
    let mut data = Vec::with_capacity(samples);
    
    for i in 0..samples {
        let t = i as f32 / sample_rate;
        let frequency = 60.0 * (1.0 - t * 5.0).max(0.1); // Frequency sweep
        let envelope = (-t * 15.0).exp(); // Exponential decay
        let sample = (2.0 * std::f32::consts::PI * frequency * t).sin() * envelope * 0.8;
        data.push(sample);
    }
    
    data
}

fn generate_snare(sample_rate: f32, duration: f32) -> Vec<f32> {
    let samples = (sample_rate * duration) as usize;
    let mut data = Vec::with_capacity(samples);
    
    for i in 0..samples {
        let t = i as f32 / sample_rate;
        let envelope = (-t * 20.0).exp(); // Fast decay
        let noise = (rand::random::<f32>() - 0.5) * 2.0; // White noise
        let tone = (2.0 * std::f32::consts::PI * 200.0 * t).sin() * 0.3; // Tonal component
        let sample = (noise * 0.7 + tone) * envelope * 0.6;
        data.push(sample);
    }
    
    data
}

fn generate_hihat(sample_rate: f32, duration: f32) -> Vec<f32> {
    let samples = (sample_rate * duration) as usize;
    let mut data = Vec::with_capacity(samples);
    
    for i in 0..samples {
        let t = i as f32 / sample_rate;
        let envelope = (-t * 30.0).exp(); // Very fast decay
        let noise = (rand::random::<f32>() - 0.5) * 2.0; // White noise
        // Simple high-pass filter effect
        let filtered_noise = if i > 0 {
            noise - data[i - 1] * 0.9
        } else {
            noise
        };
        let sample = filtered_noise * envelope * 0.4;
        data.push(sample);
    }
    
    data
}

fn generate_crash(sample_rate: f32, duration: f32) -> Vec<f32> {
    let samples = (sample_rate * duration) as usize;
    let mut data = Vec::with_capacity(samples);
    
    for i in 0..samples {
        let t = i as f32 / sample_rate;
        let envelope = (-t * 1.5).exp(); // Long decay
        let noise = (rand::random::<f32>() - 0.5) * 2.0; // White noise
        
        // Multiple frequency components for metallic sound
        let metallic = (2.0 * std::f32::consts::PI * 5000.0 * t).sin() * 0.1
                     + (2.0 * std::f32::consts::PI * 7000.0 * t).sin() * 0.08
                     + (2.0 * std::f32::consts::PI * 9000.0 * t).sin() * 0.06;
        
        // High-pass filter for bright metallic sound
        let filtered_noise = if i > 2 {
            noise - data[i - 1] * 0.7 - data[i - 2] * 0.2
        } else if i > 0 {
            noise - data[i - 1] * 0.7
        } else {
            noise
        };
        
        let sample = (filtered_noise * 0.7 + metallic) * envelope * 0.5;
        data.push(sample);
    }
    
    data
}

fn generate_open_hihat(sample_rate: f32, duration: f32) -> Vec<f32> {
    let samples = (sample_rate * duration) as usize;
    let mut data = Vec::with_capacity(samples);
    
    for i in 0..samples {
        let t = i as f32 / sample_rate;
        let envelope = (-t * 8.0).exp(); // Medium decay
        let noise = (rand::random::<f32>() - 0.5) * 2.0; // White noise
        
        // Multiple high-pass filters for bright sound
        let filtered_noise = if i > 1 {
            noise - data[i - 1] * 0.8 - data[i - 2] * 0.1
        } else if i > 0 {
            noise - data[i - 1] * 0.8
        } else {
            noise
        };
        
        let sample = filtered_noise * envelope * 0.6;
        data.push(sample);
    }
    
    data
}

fn generate_clap(sample_rate: f32, duration: f32) -> Vec<f32> {
    let samples = (sample_rate * duration) as usize;
    let mut data = Vec::with_capacity(samples);
    
    for i in 0..samples {
        let t = i as f32 / sample_rate;
        
        // Multiple bursts to simulate hand clap
        let burst1 = if t < 0.01 { (-t * 200.0).exp() } else { 0.0 };
        let burst2 = if t > 0.015 && t < 0.025 { (-(t - 0.015) * 200.0).exp() } else { 0.0 };
        let burst3 = if t > 0.03 && t < 0.04 { (-(t - 0.03) * 200.0).exp() } else { 0.0 };
        let main_envelope = if t > 0.045 { (-(t - 0.045) * 15.0).exp() } else { 0.0 };
        
        let envelope = burst1 + burst2 + burst3 + main_envelope;
        let noise = (rand::random::<f32>() - 0.5) * 2.0; // White noise
        
        // Band-pass filter for clap-like frequency content
        let filtered_noise = if i > 2 {
            noise * 0.8 - data[i - 1] * 0.3 + data[i - 2] * 0.1
        } else {
            noise * 0.8
        };
        
        let sample = filtered_noise * envelope * 0.7;
        data.push(sample);
    }
    
    data
}

fn generate_rimshot(sample_rate: f32, duration: f32) -> Vec<f32> {
    let samples = (sample_rate * duration) as usize;
    let mut data = Vec::with_capacity(samples);
    
    for i in 0..samples {
        let t = i as f32 / sample_rate;
        let envelope = (-t * 40.0).exp(); // Very sharp decay
        
        // Sharp attack with high frequency click
        let click = (2.0 * std::f32::consts::PI * 2000.0 * t).sin() * 0.5;
        let noise = (rand::random::<f32>() - 0.5) * 2.0 * 0.3; // Less noise, more tone
        
        // High-pass filter for crisp sound
        let filtered = if i > 0 {
            (click + noise) - data[i - 1] * 0.7
        } else {
            click + noise
        };
        
        let sample = filtered * envelope * 0.8;
        data.push(sample);
    }
    
    data
}

fn generate_tom(sample_rate: f32, duration: f32) -> Vec<f32> {
    let samples = (sample_rate * duration) as usize;
    let mut data = Vec::with_capacity(samples);
    
    for i in 0..samples {
        let t = i as f32 / sample_rate;
        let envelope = (-t * 6.0).exp(); // Medium-slow decay
        
        // Pitched drum with frequency sweep
        let base_freq = 80.0;
        let frequency = base_freq * (1.0 - t * 2.0).max(0.3); // Pitch bend down
        let tone = (2.0 * std::f32::consts::PI * frequency * t).sin();
        
        // Add some overtones
        let overtone1 = (2.0 * std::f32::consts::PI * frequency * 1.5 * t).sin() * 0.3;
        let overtone2 = (2.0 * std::f32::consts::PI * frequency * 2.2 * t).sin() * 0.15;
        
        // Small amount of noise for texture
        let noise = (rand::random::<f32>() - 0.5) * 2.0 * 0.1;
        
        let sample = (tone + overtone1 + overtone2 + noise) * envelope * 0.7;
        data.push(sample);
    }
    
    data
}

mod rand {
    use std::cell::Cell;
    
    thread_local! {
        static RNG_STATE: Cell<u32> = const { Cell::new(1) };
    }
    
    pub fn random<T>() -> T
    where
        T: From<f32>,
    {
        RNG_STATE.with(|state| {
            let mut x = state.get();
            x ^= x << 13;
            x ^= x >> 17;
            x ^= x << 5;
            state.set(x);
            T::from((x as f32) / (u32::MAX as f32))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sample_creation() {
        let data = vec![0.0, 0.5, -0.5, 1.0];
        let sample = Sample::from_data(data.clone(), 44100, 1);
        assert_eq!(sample.data, data);
        assert_eq!(sample.sample_rate, 44100);
        assert_eq!(sample.channels, 1);
    }

    #[test]
    fn test_sample_bank_default_loading() {
        let mut sample_bank = SampleBank::new();
        assert!(sample_bank.list_samples().is_empty(), "New sample bank should be empty");
        
        // Load default samples
        sample_bank.load_default_samples();
        
        let samples = sample_bank.list_samples();
        assert!(!samples.is_empty(), "Sample bank should have samples after loading defaults");
        
        // Check specific samples exist
        let expected_samples = vec!["kick", "snare", "hihat", "crash", "open_hihat", "clap", "rimshot", "tom"];
        for sample_name in expected_samples {
            assert!(samples.contains(&&sample_name.to_string()), 
                   "Should have sample: {}", sample_name);
            
            // Verify sample exists and has data
            let sample = sample_bank.get_sample(sample_name).unwrap();
            assert!(!sample.data.is_empty(), "Sample '{}' should have audio data", sample_name);
            assert!(sample.data.len() > 1000, "Sample '{}' should have reasonable length", sample_name);
            
            // Check for non-zero audio data
            let max_sample = sample.data.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
            assert!(max_sample > 0.0, "Sample '{}' should have non-zero audio data", sample_name);
        }
        
        println!("✅ Sample bank loads {} samples successfully", samples.len());
    }

    #[test] 
    fn test_sample_audio_characteristics() {
        let mut sample_bank = SampleBank::new();
        sample_bank.load_default_samples();
        
        // Test kick sample characteristics
        let kick = sample_bank.get_sample("kick").unwrap();
        assert!(kick.data.len() > 10000, "Kick should be reasonably long");
        
        // Check kick has strong initial transient (first samples should be significant)
        let initial_energy: f32 = kick.data.iter().take(1000).map(|s| s.abs()).sum();
        assert!(initial_energy > 10.0, "Kick should have strong initial transient");
        
        // Test hihat sample characteristics  
        let hihat = sample_bank.get_sample("hihat").unwrap();
        assert!(hihat.data.len() > 1000, "Hi-hat should have reasonable length");
        
        // Check hihat has sharp attack (high frequency content)
        let max_hihat = hihat.data.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
        assert!(max_hihat > 0.1, "Hi-hat should have sufficient amplitude");
        
        println!("✅ Sample audio characteristics verified");
    }

    #[test]
    fn test_sample_bank_operations() {
        let mut bank = SampleBank::new();
        let sample = Sample::from_data(vec![0.1, 0.2, 0.3], 44100, 1);
        
        bank.add_sample("test".to_string(), sample);
        assert!(bank.get_sample("test").is_some());
        assert!(bank.get_sample("nonexistent").is_none());
    }

    #[test]
    fn test_new_sample_synthesis_functions() {
        let sample_rate = 44100.0;
        let duration = 0.1;
        
        // Test all new synthesis functions generate non-empty data
        let crash = generate_crash(sample_rate, 2.0);
        let open_hihat = generate_open_hihat(sample_rate, duration);
        let clap = generate_clap(sample_rate, duration);
        let rimshot = generate_rimshot(sample_rate, duration);
        let tom = generate_tom(sample_rate, 0.6);
        
        assert!(!crash.is_empty());
        assert!(!open_hihat.is_empty());
        assert!(!clap.is_empty());
        assert!(!rimshot.is_empty());
        assert!(!tom.is_empty());
        
        // Test sample lengths are reasonable
        let expected_crash_len = (sample_rate * 2.0) as usize;
        let expected_short_len = (sample_rate * duration) as usize;
        
        assert_eq!(crash.len(), expected_crash_len);
        assert_eq!(open_hihat.len(), expected_short_len);
        assert_eq!(clap.len(), expected_short_len);
        assert_eq!(rimshot.len(), expected_short_len);
    }

    #[test]
    fn test_default_samples_load_correctly() {
        let mut bank = SampleBank::new();
        bank.load_default_samples();
        
        // Verify all 8 samples are loaded (3 original + 5 new)
        assert!(bank.get_sample("kick").is_some());
        assert!(bank.get_sample("snare").is_some());
        assert!(bank.get_sample("hihat").is_some());
        assert!(bank.get_sample("crash").is_some());
        assert!(bank.get_sample("open_hihat").is_some());
        assert!(bank.get_sample("clap").is_some());
        assert!(bank.get_sample("rimshot").is_some());
        assert!(bank.get_sample("tom").is_some());
    }

    #[test]
    fn test_synthesis_functions_produce_valid_audio() {
        let sample_rate = 44100.0;
        let samples = generate_kick(sample_rate, 0.1);
        
        // Verify no NaN or infinite values
        for sample in &samples {
            assert!(sample.is_finite());
        }
        
        // Verify audio levels are reasonable (not clipping)
        let max_amplitude = samples.iter().map(|x| x.abs()).fold(0.0, f32::max);
        assert!(max_amplitude <= 1.0, "Audio should not clip");
    }
}