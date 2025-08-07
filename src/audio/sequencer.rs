use super::SampleBank;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeSignature {
    pub numerator: u8,   // Number of beats per measure (1-32)
    pub denominator: u8, // Note value for one beat (1, 2, 4, 8, 16, 32)
}

impl TimeSignature {
    pub fn new(numerator: u8, denominator: u8) -> Result<Self, String> {
        // Validate numerator (beats per measure)
        if numerator == 0 || numerator > 32 {
            return Err(format!("Invalid numerator: {}. Must be 1-32", numerator));
        }

        // Validate denominator (note value)
        if !denominator.is_power_of_two() || denominator == 0 || denominator > 32 {
            return Err(format!(
                "Invalid denominator: {}. Must be power of 2 (1, 2, 4, 8, 16, 32)",
                denominator
            ));
        }

        Ok(TimeSignature {
            numerator,
            denominator,
        })
    }

    // Common time signature presets
    pub fn four_four() -> Self {
        TimeSignature {
            numerator: 4,
            denominator: 4,
        }
    }
    pub fn three_four() -> Self {
        TimeSignature {
            numerator: 3,
            denominator: 4,
        }
    }
    pub fn five_four() -> Self {
        TimeSignature {
            numerator: 5,
            denominator: 4,
        }
    }
    pub fn six_eight() -> Self {
        TimeSignature {
            numerator: 6,
            denominator: 8,
        }
    }
    pub fn seven_eight() -> Self {
        TimeSignature {
            numerator: 7,
            denominator: 8,
        }
    }
    pub fn nine_eight() -> Self {
        TimeSignature {
            numerator: 9,
            denominator: 8,
        }
    }
    pub fn twelve_eight() -> Self {
        TimeSignature {
            numerator: 12,
            denominator: 8,
        }
    }

    // Calculate steps per beat for a given loop length
    pub fn steps_per_beat(&self, loop_length: usize) -> f32 {
        // For 4/4 with 16 steps: 16 / 4 = 4 steps per beat
        // For 3/4 with 12 steps: 12 / 3 = 4 steps per beat
        // For 5/4 with 20 steps: 20 / 5 = 4 steps per beat
        loop_length as f32 / self.numerator as f32
    }

    // Calculate optimal loop length for this time signature
    pub fn optimal_loop_length(&self, base_subdivision: u8) -> usize {
        // Base subdivision is typically 4 (16th notes) or 8 (8th notes)
        let steps_per_beat = base_subdivision as usize;
        self.numerator as usize * steps_per_beat
    }

    // Get beat index for a given step (0-based)
    pub fn beat_for_step(&self, step: usize, loop_length: usize) -> usize {
        let steps_per_beat = self.steps_per_beat(loop_length);
        (step as f32 / steps_per_beat).floor() as usize % self.numerator as usize
    }

    // Check if a step is a beat boundary
    pub fn is_beat_boundary(&self, step: usize, loop_length: usize) -> bool {
        let steps_per_beat = self.steps_per_beat(loop_length);
        (step as f32 % steps_per_beat).abs() < 0.001 // Account for floating point precision
    }

    // Check if a step is the downbeat (first beat of measure)
    pub fn is_downbeat(&self, step: usize, loop_length: usize) -> bool {
        self.is_beat_boundary(step, loop_length) && self.beat_for_step(step, loop_length) == 0
    }

    // Get display string for time signature
    pub fn display_string(&self) -> String {
        format!("{}/{}", self.numerator, self.denominator)
    }

    // Get musical step label (beat.subdivision format)
    pub fn step_label(&self, step: usize, loop_length: usize) -> String {
        let steps_per_beat = self.steps_per_beat(loop_length);
        let beat_number = (step as f32 / steps_per_beat).floor() as usize + 1; // 1-indexed
        let subdivision = (step as f32 % steps_per_beat).floor() as usize + 1; // 1-indexed

        if steps_per_beat >= 4.0 {
            // Show subdivision for detailed patterns (e.g., "1.1", "1.2", "1.3", "1.4")
            format!("{}.{}", beat_number, subdivision)
        } else {
            // Show just beat number for simple patterns (e.g., "1", "2", "3")
            format!("{}", beat_number)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Step {
    pub active: bool,
    pub velocity: f32,
}

impl Step {
    pub fn new() -> Self {
        Step {
            active: false,
            velocity: 1.0,
        }
    }

    pub fn with_velocity(velocity: f32) -> Self {
        Step {
            active: true,
            velocity: velocity.clamp(0.0, 1.0),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pattern {
    pub name: String,
    pub steps: Vec<Step>,
    pub sample_name: String,
}

impl Pattern {
    pub fn new(name: String, sample_name: String, num_steps: usize) -> Self {
        Pattern {
            name,
            steps: vec![Step::new(); num_steps],
            sample_name,
        }
    }

    pub fn set_step(&mut self, step_index: usize, step: Step) {
        if step_index < self.steps.len() {
            self.steps[step_index] = step;
        }
    }

    pub fn toggle_step(&mut self, step_index: usize) {
        if step_index < self.steps.len() {
            self.steps[step_index].active = !self.steps[step_index].active;
        }
    }

    pub fn clear(&mut self) {
        for step in &mut self.steps {
            step.active = false;
        }
    }

    pub fn length(&self) -> usize {
        self.steps.len()
    }

    pub fn resize(&mut self, new_length: usize) {
        match new_length.cmp(&self.steps.len()) {
            std::cmp::Ordering::Greater => {
                // Extend with empty steps
                let additional_steps = new_length - self.steps.len();
                for _ in 0..additional_steps {
                    self.steps.push(Step::new());
                }
            }
            std::cmp::Ordering::Less => {
                // Truncate to new length
                self.steps.truncate(new_length);
            }
            std::cmp::Ordering::Equal => {
                // No change needed
            }
        }
    }
}

pub struct Voice {
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

pub struct Sequencer {
    current_step: usize,
    samples_per_step: usize,
    sample_counter: usize,
    playing: bool,
    voices: Vec<Voice>,
    bpm: f32,
    sample_rate: f32,
    loop_length: usize,
    time_signature: TimeSignature,
    patterns: Vec<Pattern>,
}

impl Sequencer {
    pub fn new(sample_rate: f32, bpm: f32) -> Self {
        let mut sequencer = Sequencer {
            current_step: 0,
            samples_per_step: 0,
            sample_counter: 0,
            playing: false,
            voices: Vec::new(),
            bpm,
            sample_rate,
            loop_length: 16, // Default to 16 steps for backward compatibility
            time_signature: TimeSignature::four_four(), // Default to 4/4 time
            patterns: Vec::new(),
        };

        sequencer.update_timing();

        // Initialize voices
        for _ in 0..16 {
            sequencer.voices.push(Voice::new());
        }

        sequencer
    }

    pub fn set_bpm(&mut self, bpm: f32) {
        self.bpm = bpm;
        self.update_timing();
    }

    pub fn get_bpm(&self) -> f32 {
        self.bpm
    }

    pub fn play(&mut self) {
        self.playing = true;
    }

    pub fn stop(&mut self) {
        self.playing = false;
        self.current_step = 0;
        self.sample_counter = 0;
        // Stop all voices
        for voice in &mut self.voices {
            voice.active = false;
        }
    }

    pub fn pause(&mut self) {
        self.playing = false;
    }

    pub fn is_playing(&self) -> bool {
        self.playing
    }

    pub fn get_current_step(&self) -> usize {
        self.current_step
    }

    pub fn get_loop_length(&self) -> usize {
        self.loop_length
    }

    #[cfg(test)]
    pub fn get_sample_counter(&self) -> usize {
        self.sample_counter
    }

    pub fn get_time_signature(&self) -> TimeSignature {
        self.time_signature
    }

    pub fn set_time_signature(&mut self, time_signature: TimeSignature) {
        self.time_signature = time_signature;

        // Optionally adjust loop length to match time signature optimally
        let optimal_length = time_signature.optimal_loop_length(4); // Using 16th note subdivision
        if optimal_length != self.loop_length {
            self.set_loop_length(optimal_length);
        }
    }

    pub fn set_loop_length(&mut self, new_length: usize) {
        if new_length > 0 && new_length <= 64 {
            self.loop_length = new_length;

            // Reset current step if it's beyond the new length
            if self.current_step >= new_length {
                self.current_step = 0;
                self.sample_counter = 0;
            }
        }
    }

    fn update_timing(&mut self) {
        // Calculate samples per 16th note
        let beats_per_second = self.bpm / 60.0;
        let steps_per_second = beats_per_second * 4.0; // 16th notes
        self.samples_per_step = (self.sample_rate / steps_per_second) as usize;
    }

    pub fn process_audio_with_patterns(
        &mut self,
        output: &mut [f32],
        sample_bank: &SampleBank,
        patterns: &[Pattern],
    ) {
        // Clear output buffer
        for sample in output.iter_mut() {
            *sample = 0.0;
        }

        if !self.playing {
            return;
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

    fn trigger_current_step(&mut self, _sample_bank: &SampleBank, patterns: &[Pattern]) {
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

    pub fn advance_step(&mut self) {
        self.current_step = (self.current_step + 1) % self.loop_length;
    }

    // Pattern management methods
    pub fn initialize_default_patterns(&mut self) {
        self.patterns.clear();

        // Create default patterns with current loop length and some active steps
        let patterns_data = vec![
            ("Kick", "kick", vec![0, 4, 8, 12]),     // Kick on downbeats
            ("Snare", "snare", vec![4, 12]),         // Snare on 2 and 4
            ("Hi-Hat", "hihat", vec![2, 6, 10, 14]), // Hi-hat on off-beats
            ("Crash", "crash", vec![]),              // No active steps by default
            ("Open Hi-Hat", "open_hihat", vec![]),   // No active steps by default
            ("Clap", "clap", vec![]),                // No active steps by default
            ("Rim Shot", "rimshot", vec![]),         // No active steps by default
            ("Tom", "tom", vec![]),                  // No active steps by default
        ];

        for (name, sample_name, active_steps) in patterns_data {
            let mut pattern =
                Pattern::new(name.to_string(), sample_name.to_string(), self.loop_length);

            // Activate specified steps
            for step_index in active_steps {
                if step_index < pattern.steps.len() {
                    pattern.steps[step_index].active = true;
                }
            }

            self.patterns.push(pattern);
        }
    }

    pub fn get_patterns(&self) -> &Vec<Pattern> {
        &self.patterns
    }

    pub fn get_patterns_mut(&mut self) -> &mut Vec<Pattern> {
        &mut self.patterns
    }

    // Regular sequencer playback method (non-timeline mode)
    pub fn process_audio(&mut self, output: &mut [f32], sample_bank: &SampleBank) {
        self.process_audio_with_patterns(output, sample_bank, &self.patterns.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sequencer_with_additional_samples() {
        let mut sequencer = Sequencer::new(44100.0, 120.0);
        sequencer.initialize_default_patterns();

        // Verify all 8 patterns are created (3 original + 5 new)
        let patterns = sequencer.get_patterns();
        assert_eq!(patterns.len(), 8);

        // Verify pattern names match expected samples
        let expected_names = vec![
            "Kick",
            "Snare",
            "Hi-Hat",
            "Crash",
            "Open Hi-Hat",
            "Clap",
            "Rim Shot",
            "Tom",
        ];
        for (i, expected_name) in expected_names.iter().enumerate() {
            assert_eq!(&patterns[i].name, expected_name);
        }

        // Verify sample names are correctly mapped
        assert_eq!(&patterns[3].sample_name, "crash");
        assert_eq!(&patterns[4].sample_name, "open_hihat");
        assert_eq!(&patterns[5].sample_name, "clap");
        assert_eq!(&patterns[6].sample_name, "rimshot");
        assert_eq!(&patterns[7].sample_name, "tom");
    }

    #[test]
    fn test_pattern_step_manipulation() {
        let mut pattern = Pattern::new("Test".to_string(), "test_sample".to_string(), 16);

        // Test step toggling
        assert!(!pattern.steps[0].active);
        pattern.toggle_step(0);
        assert!(pattern.steps[0].active);
        pattern.toggle_step(0);
        assert!(!pattern.steps[0].active);

        // Test clearing
        pattern.toggle_step(0);
        pattern.toggle_step(5);
        pattern.clear();
        assert!(!pattern.steps[0].active);
        assert!(!pattern.steps[5].active);
    }

    #[test]
    fn test_backward_compatibility() {
        let mut sequencer = Sequencer::new(44100.0, 120.0);
        sequencer.initialize_default_patterns();

        // Verify original 3 samples are still in the same positions
        let patterns = sequencer.get_patterns();
        assert_eq!(&patterns[0].sample_name, "kick");
        assert_eq!(&patterns[1].sample_name, "snare");
        assert_eq!(&patterns[2].sample_name, "hihat");
    }

    #[test]
    fn test_variable_loop_lengths() {
        let mut sequencer = Sequencer::new(44100.0, 120.0);

        // Test default loop length
        assert_eq!(sequencer.get_loop_length(), 16);

        // Test setting different loop lengths
        sequencer.set_loop_length(4);
        assert_eq!(sequencer.get_loop_length(), 4);

        sequencer.set_loop_length(8);
        assert_eq!(sequencer.get_loop_length(), 8);

        sequencer.set_loop_length(32);
        assert_eq!(sequencer.get_loop_length(), 32);

        // Test boundary conditions
        sequencer.set_loop_length(1);
        assert_eq!(sequencer.get_loop_length(), 1);

        sequencer.set_loop_length(64);
        assert_eq!(sequencer.get_loop_length(), 64);

        // Test invalid values (should not change loop length)
        sequencer.set_loop_length(0);
        assert_eq!(sequencer.get_loop_length(), 64); // Should remain unchanged

        sequencer.set_loop_length(65);
        assert_eq!(sequencer.get_loop_length(), 64); // Should remain unchanged
    }

    #[test]
    fn test_pattern_resize_functionality() {
        let mut pattern = Pattern::new("Test".to_string(), "test_sample".to_string(), 16);

        // Set some steps as active
        pattern.toggle_step(0);
        pattern.toggle_step(5);
        pattern.toggle_step(10);
        pattern.toggle_step(15);

        // Test extending pattern (should preserve existing steps and add empty ones)
        pattern.resize(20);
        assert_eq!(pattern.length(), 20);
        assert!(pattern.steps[0].active);
        assert!(pattern.steps[5].active);
        assert!(pattern.steps[10].active);
        assert!(pattern.steps[15].active);
        assert!(!pattern.steps[16].active); // New steps should be inactive
        assert!(!pattern.steps[19].active);

        // Test truncating pattern (should preserve remaining steps)
        pattern.resize(8);
        assert_eq!(pattern.length(), 8);
        assert!(pattern.steps[0].active);
        assert!(pattern.steps[5].active);
        // Steps 10 and 15 should be gone now

        // Test no-op resize
        let original_length = pattern.length();
        pattern.resize(8);
        assert_eq!(pattern.length(), original_length);
    }

    #[test]
    fn test_sequencer_step_advancement_with_variable_lengths() {
        let mut sequencer = Sequencer::new(44100.0, 120.0);

        // Test with 4-step loop
        sequencer.set_loop_length(4);
        assert_eq!(sequencer.get_current_step(), 0);

        // Manually advance steps to test wrapping
        for _ in 0..3 {
            sequencer.advance_step();
        }
        assert_eq!(sequencer.get_current_step(), 3);

        // Next step should wrap to 0
        sequencer.advance_step();
        assert_eq!(sequencer.get_current_step(), 0);

        // Test with different loop length
        sequencer.set_loop_length(6);
        sequencer.advance_step(); // Should be at step 1 now
        for _ in 0..5 {
            sequencer.advance_step();
        }
        assert_eq!(sequencer.get_current_step(), 0); // Should wrap after step 5
    }

    #[test]
    fn test_current_step_reset_when_loop_length_changes() {
        let mut sequencer = Sequencer::new(44100.0, 120.0);

        // Advance to a later step
        for _ in 0..10 {
            sequencer.advance_step();
        }
        assert_eq!(sequencer.get_current_step(), 10);

        // Change to a shorter loop length that would put current_step out of bounds
        sequencer.set_loop_length(8);
        assert_eq!(sequencer.get_current_step(), 0); // Should reset to 0
        assert_eq!(sequencer.get_sample_counter(), 0); // Sample counter should also reset

        // Test changing to a longer loop length (should not reset if current step is valid)
        sequencer.advance_step();
        sequencer.advance_step();
        assert_eq!(sequencer.get_current_step(), 2);

        sequencer.set_loop_length(16);
        assert_eq!(sequencer.get_current_step(), 2); // Should not reset since 2 < 16
    }

    #[test]
    fn test_odd_time_signatures() {
        let mut sequencer = Sequencer::new(44100.0, 120.0);

        // Test various odd time signatures that should be supported
        let odd_lengths = [3, 5, 7, 9, 11, 13, 15, 17, 19];

        for &length in &odd_lengths {
            sequencer.set_loop_length(length);
            assert_eq!(
                sequencer.get_loop_length(),
                length,
                "Failed to set loop length to {}",
                length
            );

            // Test step advancement wraps correctly
            for _ in 0..length {
                sequencer.advance_step();
            }
            assert_eq!(
                sequencer.get_current_step(),
                0,
                "Step should wrap to 0 after {} steps",
                length
            );

            // Test that patterns are resized correctly
            sequencer.initialize_default_patterns();
            let patterns = sequencer.get_patterns();
            for pattern in patterns {
                assert_eq!(
                    pattern.length(),
                    length,
                    "Pattern should be resized to {}",
                    length
                );
            }
        }
    }

    #[test]
    fn test_time_signature_creation() {
        // Test valid time signatures
        assert!(TimeSignature::new(4, 4).is_ok());
        assert!(TimeSignature::new(3, 4).is_ok());
        assert!(TimeSignature::new(5, 4).is_ok());
        assert!(TimeSignature::new(6, 8).is_ok());
        assert!(TimeSignature::new(7, 8).is_ok());
        assert!(TimeSignature::new(12, 8).is_ok());
        assert!(TimeSignature::new(1, 1).is_ok());
        assert!(TimeSignature::new(32, 32).is_ok());

        // Test invalid numerators
        assert!(TimeSignature::new(0, 4).is_err());
        assert!(TimeSignature::new(33, 4).is_err());

        // Test invalid denominators (not power of 2)
        assert!(TimeSignature::new(4, 3).is_err());
        assert!(TimeSignature::new(4, 5).is_err());
        assert!(TimeSignature::new(4, 6).is_err());
        assert!(TimeSignature::new(4, 7).is_err());
        assert!(TimeSignature::new(4, 9).is_err());

        // Test invalid denominators (0 or too large)
        assert!(TimeSignature::new(4, 0).is_err());
        assert!(TimeSignature::new(4, 64).is_err());
    }

    #[test]
    fn test_time_signature_presets() {
        let four_four = TimeSignature::four_four();
        assert_eq!(four_four.numerator, 4);
        assert_eq!(four_four.denominator, 4);

        let three_four = TimeSignature::three_four();
        assert_eq!(three_four.numerator, 3);
        assert_eq!(three_four.denominator, 4);

        let five_four = TimeSignature::five_four();
        assert_eq!(five_four.numerator, 5);
        assert_eq!(five_four.denominator, 4);

        let six_eight = TimeSignature::six_eight();
        assert_eq!(six_eight.numerator, 6);
        assert_eq!(six_eight.denominator, 8);

        assert_eq!(four_four.display_string(), "4/4");
        assert_eq!(three_four.display_string(), "3/4");
        assert_eq!(five_four.display_string(), "5/4");
        assert_eq!(six_eight.display_string(), "6/8");
    }

    #[test]
    fn test_time_signature_calculations() {
        let four_four = TimeSignature::four_four();
        let three_four = TimeSignature::three_four();
        let five_four = TimeSignature::five_four();

        // Test steps per beat with 16-step patterns
        assert!((four_four.steps_per_beat(16) - 4.0).abs() < 0.001); // 16/4 = 4 steps per beat
        assert!((three_four.steps_per_beat(12) - 4.0).abs() < 0.001); // 12/3 = 4 steps per beat
        assert!((five_four.steps_per_beat(20) - 4.0).abs() < 0.001); // 20/5 = 4 steps per beat

        // Test optimal loop lengths
        assert_eq!(four_four.optimal_loop_length(4), 16); // 4 beats * 4 subdivisions = 16 steps
        assert_eq!(three_four.optimal_loop_length(4), 12); // 3 beats * 4 subdivisions = 12 steps
        assert_eq!(five_four.optimal_loop_length(4), 20); // 5 beats * 4 subdivisions = 20 steps

        // Test beat boundary detection
        assert!(four_four.is_beat_boundary(0, 16)); // Step 0 is beat 1
        assert!(four_four.is_beat_boundary(4, 16)); // Step 4 is beat 2
        assert!(four_four.is_beat_boundary(8, 16)); // Step 8 is beat 3
        assert!(four_four.is_beat_boundary(12, 16)); // Step 12 is beat 4
        assert!(!four_four.is_beat_boundary(1, 16)); // Step 1 is not a beat boundary
        assert!(!four_four.is_beat_boundary(3, 16)); // Step 3 is not a beat boundary

        // Test downbeat detection (only beat 1)
        assert!(four_four.is_downbeat(0, 16)); // Step 0 is the downbeat
        assert!(!four_four.is_downbeat(4, 16)); // Step 4 is beat 2, not downbeat
        assert!(!four_four.is_downbeat(8, 16)); // Step 8 is beat 3, not downbeat
        assert!(!four_four.is_downbeat(12, 16)); // Step 12 is beat 4, not downbeat

        // Test beat index calculation
        assert_eq!(four_four.beat_for_step(0, 16), 0); // Step 0 is beat 0 (1st beat)
        assert_eq!(four_four.beat_for_step(4, 16), 1); // Step 4 is beat 1 (2nd beat)
        assert_eq!(four_four.beat_for_step(8, 16), 2); // Step 8 is beat 2 (3rd beat)
        assert_eq!(four_four.beat_for_step(12, 16), 3); // Step 12 is beat 3 (4th beat)

        // Test 3/4 time signature
        assert_eq!(three_four.beat_for_step(0, 12), 0); // Step 0 is beat 0 (1st beat)
        assert_eq!(three_four.beat_for_step(4, 12), 1); // Step 4 is beat 1 (2nd beat)
        assert_eq!(three_four.beat_for_step(8, 12), 2); // Step 8 is beat 2 (3rd beat)
        assert!(three_four.is_downbeat(0, 12)); // Step 0 is downbeat
        assert!(!three_four.is_downbeat(4, 12)); // Step 4 is not downbeat
        assert!(!three_four.is_downbeat(8, 12)); // Step 8 is not downbeat
    }

    #[test]
    fn test_sequencer_time_signature_integration() {
        let mut sequencer = Sequencer::new(44100.0, 120.0);

        // Test default time signature
        let default_ts = sequencer.get_time_signature();
        assert_eq!(default_ts.numerator, 4);
        assert_eq!(default_ts.denominator, 4);

        // Test setting different time signatures
        sequencer.set_time_signature(TimeSignature::three_four());
        let ts = sequencer.get_time_signature();
        assert_eq!(ts.numerator, 3);
        assert_eq!(ts.denominator, 4);

        // Verify loop length was automatically adjusted to optimal
        assert_eq!(sequencer.get_loop_length(), 12); // 3 beats * 4 subdivisions

        // Test 5/4 time signature
        sequencer.set_time_signature(TimeSignature::five_four());
        assert_eq!(sequencer.get_loop_length(), 20); // 5 beats * 4 subdivisions

        // Test that patterns are resized when time signature changes
        sequencer.initialize_default_patterns();
        let patterns = sequencer.get_patterns();
        for pattern in patterns {
            assert_eq!(
                pattern.length(),
                20,
                "Pattern should be resized to match new time signature"
            );
        }
    }

    #[test]
    fn test_default_patterns_have_active_steps() {
        let mut sequencer = Sequencer::new(44100.0, 120.0);
        sequencer.initialize_default_patterns();

        let patterns = sequencer.get_patterns();
        assert_eq!(patterns.len(), 8, "Should have 8 default patterns");

        // Test kick pattern (should have steps 0, 4, 8, 12 active)
        let kick_pattern = &patterns[0];
        assert_eq!(kick_pattern.name, "Kick");
        assert_eq!(kick_pattern.sample_name, "kick");

        let active_steps: Vec<usize> = kick_pattern
            .steps
            .iter()
            .enumerate()
            .filter(|(_, step)| step.active)
            .map(|(i, _)| i)
            .collect();

        assert_eq!(
            active_steps,
            vec![0, 4, 8, 12],
            "Kick should be active on downbeats"
        );

        // Test snare pattern (should have steps 4, 12 active)
        let snare_pattern = &patterns[1];
        assert_eq!(snare_pattern.name, "Snare");
        assert_eq!(snare_pattern.sample_name, "snare");

        let snare_active_steps: Vec<usize> = snare_pattern
            .steps
            .iter()
            .enumerate()
            .filter(|(_, step)| step.active)
            .map(|(i, _)| i)
            .collect();

        assert_eq!(
            snare_active_steps,
            vec![4, 12],
            "Snare should be active on beats 2 & 4"
        );

        // Test hi-hat pattern (should have steps 2, 6, 10, 14 active)
        let hihat_pattern = &patterns[2];
        assert_eq!(hihat_pattern.name, "Hi-Hat");
        assert_eq!(hihat_pattern.sample_name, "hihat");

        let hihat_active_steps: Vec<usize> = hihat_pattern
            .steps
            .iter()
            .enumerate()
            .filter(|(_, step)| step.active)
            .map(|(i, _)| i)
            .collect();

        assert_eq!(
            hihat_active_steps,
            vec![2, 6, 10, 14],
            "Hi-hat should be active on off-beats"
        );

        println!("✅ Default patterns have correct active steps");
    }

    #[test]
    fn test_audio_processing_with_active_patterns() {
        let mut sequencer = Sequencer::new(44100.0, 120.0);
        sequencer.initialize_default_patterns();

        let mut sample_bank = SampleBank::new();
        sample_bank.load_default_samples();

        // Start sequencer
        sequencer.play();
        assert!(sequencer.is_playing(), "Sequencer should be playing");

        // Create audio buffer
        let buffer_size = 1024;
        let mut audio_buffer = vec![0.0f32; buffer_size];

        // Process several audio buffers to trigger some steps
        let mut total_max_sample = 0.0f32;
        let mut step_triggers = 0;
        let initial_step = sequencer.get_current_step();

        // Need enough buffers to advance steps: 5512 samples/step ÷ 1024 samples/buffer ≈ 6 buffers per step
        // Process enough to advance at least 2 steps
        for i in 0..15 {
            // Process 15 buffers (should advance ~2-3 steps)
            let step_before = sequencer.get_current_step();

            // Clear buffer
            for sample in audio_buffer.iter_mut() {
                *sample = 0.0;
            }

            // Process audio
            sequencer.process_audio(&mut audio_buffer, &sample_bank);

            // Check if any audio was generated
            let max_sample = audio_buffer.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
            if max_sample > total_max_sample {
                total_max_sample = max_sample;
            }

            let step_after = sequencer.get_current_step();
            if step_after != step_before {
                step_triggers += 1;
                println!(
                    "Step advanced from {} to {} on buffer {}",
                    step_before, step_after, i
                );
            }
        }

        println!("Max audio sample detected: {}", total_max_sample);
        println!("Step changes detected: {}", step_triggers);

        assert!(
            step_triggers > 0,
            "Sequencer should advance steps during processing"
        );
        assert!(
            total_max_sample > 0.0,
            "Audio processing should generate non-zero samples"
        );

        println!("✅ Audio processing generates output");
    }

    #[test]
    fn test_voice_triggering() {
        let mut sequencer = Sequencer::new(44100.0, 120.0);
        sequencer.initialize_default_patterns();

        let sample_bank = SampleBank::new();
        // Note: sample_bank is empty, but we can still test voice allocation

        let patterns = sequencer.get_patterns().clone(); // Clone to avoid borrow issues

        // Manually trigger step 0 (should have kick active)
        sequencer.current_step = 0;
        sequencer.trigger_current_step(&sample_bank, &patterns);

        // Check if any voices were activated
        let active_voices = sequencer.voices.iter().filter(|v| v.active).count();
        assert!(
            active_voices > 0,
            "At least one voice should be active after triggering step 0"
        );

        println!(
            "✅ Voice triggering works - {} voices active",
            active_voices
        );
    }

    #[test]
    fn test_sequencer_timing_calculations() {
        let mut sequencer = Sequencer::new(44100.0, 120.0);

        // Check timing calculations
        println!("Sample rate: {}", sequencer.sample_rate);
        println!("BPM: {}", sequencer.bpm);
        println!("Samples per step: {}", sequencer.samples_per_step);

        // At 120 BPM, 16th notes should be:
        // 120 BPM = 2 beats per second
        // 16th notes = 2 * 4 = 8 steps per second
        // At 44100 Hz: 44100 / 8 = 5512.5 samples per step
        let expected_samples_per_step = 44100.0 / (120.0 / 60.0 * 4.0);
        println!("Expected samples per step: {}", expected_samples_per_step);

        assert!(
            (sequencer.samples_per_step as f32 - expected_samples_per_step).abs() < 1.0,
            "Timing calculation should be approximately correct"
        );

        // Test step advancement with small buffer
        sequencer.play();
        let mut sample_bank = SampleBank::new();
        sample_bank.load_default_samples();

        let small_buffer_size = 512; // Much smaller than samples_per_step
        let mut audio_buffer = vec![0.0f32; small_buffer_size];

        let initial_step = sequencer.get_current_step();
        let initial_counter = sequencer.sample_counter;

        // Process multiple small buffers
        for i in 0..20 {
            sequencer.process_audio(&mut audio_buffer, &sample_bank);
            let current_counter = sequencer.sample_counter;
            let current_step = sequencer.get_current_step();

            if i == 0 {
                println!(
                    "After buffer {}: step={}, counter={}",
                    i, current_step, current_counter
                );
            }
            if current_step != initial_step {
                println!(
                    "✅ Step advanced from {} to {} after {} buffers",
                    initial_step,
                    current_step,
                    i + 1
                );
                return; // Success!
            }
        }

        println!(
            "Final: step={}, counter={}, samples_per_step={}",
            sequencer.get_current_step(),
            sequencer.sample_counter,
            sequencer.samples_per_step
        );

        panic!("Sequencer did not advance steps after processing 20 buffers");
    }

    #[test]
    fn test_musical_step_labeling() {
        let four_four = TimeSignature::four_four();
        let three_four = TimeSignature::three_four();

        // Test 4/4 with 16 steps (4 steps per beat)
        assert_eq!(four_four.step_label(0, 16), "1.1"); // Beat 1, subdivision 1
        assert_eq!(four_four.step_label(1, 16), "1.2"); // Beat 1, subdivision 2
        assert_eq!(four_four.step_label(2, 16), "1.3"); // Beat 1, subdivision 3
        assert_eq!(four_four.step_label(3, 16), "1.4"); // Beat 1, subdivision 4
        assert_eq!(four_four.step_label(4, 16), "2.1"); // Beat 2, subdivision 1
        assert_eq!(four_four.step_label(8, 16), "3.1"); // Beat 3, subdivision 1
        assert_eq!(four_four.step_label(12, 16), "4.1"); // Beat 4, subdivision 1
        assert_eq!(four_four.step_label(15, 16), "4.4"); // Beat 4, subdivision 4

        // Test 3/4 with 12 steps (4 steps per beat)
        assert_eq!(three_four.step_label(0, 12), "1.1"); // Beat 1, subdivision 1
        assert_eq!(three_four.step_label(4, 12), "2.1"); // Beat 2, subdivision 1
        assert_eq!(three_four.step_label(8, 12), "3.1"); // Beat 3, subdivision 1
        assert_eq!(three_four.step_label(11, 12), "3.4"); // Beat 3, subdivision 4

        // Test 3/4 with 6 steps (2 steps per beat - should show just beat numbers)
        assert_eq!(three_four.step_label(0, 6), "1"); // Beat 1
        assert_eq!(three_four.step_label(1, 6), "1"); // Beat 1, subdivision 2
        assert_eq!(three_four.step_label(2, 6), "2"); // Beat 2
        assert_eq!(three_four.step_label(4, 6), "3"); // Beat 3

        // Test 3/4 with 3 steps (1 step per beat)
        assert_eq!(three_four.step_label(0, 3), "1"); // Beat 1
        assert_eq!(three_four.step_label(1, 3), "2"); // Beat 2
        assert_eq!(three_four.step_label(2, 3), "3"); // Beat 3
    }
}
