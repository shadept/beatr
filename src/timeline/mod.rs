use std::sync::atomic::{AtomicUsize, Ordering};
use crate::audio::{TimeSignature, sequencer::Pattern};

// Simple ID generator for timeline segments
static SEGMENT_ID_COUNTER: AtomicUsize = AtomicUsize::new(0);

fn generate_segment_id() -> String {
    let id = SEGMENT_ID_COUNTER.fetch_add(1, Ordering::SeqCst);
    format!("segment_{}", id)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PlaybackState {
    Stopped,
    Playing,
    Paused,
}

#[derive(Debug, Clone)]
pub struct TimelineSegment {
    pub id: String,
    pub start_time: f64,        // Seconds from timeline start
    pub duration: f64,          // Segment duration in seconds
    pub pattern_id: String,     // Reference to original pattern by name (for display)
    pub patterns: Vec<Pattern>, // Independent copy of all pattern data (kick, snare, hihat, etc.)
    pub loop_count: usize,      // How many times to repeat this pattern
    pub time_signature: TimeSignature,
    pub bpm: f32,
}

impl TimelineSegment {
    pub fn new(pattern_id: String, patterns: Vec<Pattern>, start_time: f64, loop_count: usize, time_signature: TimeSignature, bpm: f32) -> Self {
        let id = generate_segment_id();
        
        // Calculate duration based on loop count, time signature, and BPM
        // Duration = (beats_per_loop * loop_count) / (BPM / 60)
        let beats_per_loop = time_signature.numerator as f64;
        let total_beats = beats_per_loop * loop_count as f64;
        let beats_per_second = bpm as f64 / 60.0;
        let duration = total_beats / beats_per_second;
        
        TimelineSegment {
            id,
            start_time,
            duration,
            pattern_id,
            patterns,
            loop_count,
            time_signature,
            bpm,
        }
    }
    
    pub fn end_time(&self) -> f64 {
        self.start_time + self.duration
    }
    
    pub fn contains_time(&self, time: f64) -> bool {
        time >= self.start_time && time < self.end_time()
    }
    
    pub fn update_duration(&mut self) {
        // Recalculate duration when loop count, time signature, or BPM changes
        let beats_per_loop = self.time_signature.numerator as f64;
        let total_beats = beats_per_loop * self.loop_count as f64;
        let beats_per_second = self.bpm as f64 / 60.0;
        self.duration = total_beats / beats_per_second;
    }
    
    pub fn set_loop_count(&mut self, loop_count: usize) {
        self.loop_count = loop_count.max(1);
        self.update_duration();
    }
    
    pub fn set_bpm(&mut self, bpm: f32) {
        self.bpm = bpm.max(60.0).min(300.0); // Reasonable BPM range
        self.update_duration();
    }
    
    pub fn set_time_signature(&mut self, time_signature: TimeSignature) {
        self.time_signature = time_signature;
        self.update_duration();
    }
}

#[derive(Debug, Clone)]
pub struct Timeline {
    pub segments: Vec<TimelineSegment>,
    pub current_position: f64,  // Current playback position in seconds
    pub playback_state: PlaybackState,
}

impl Timeline {
    pub fn new() -> Self {
        Timeline {
            segments: Vec::new(),
            current_position: 0.0,
            playback_state: PlaybackState::Stopped,
        }
    }
    
    pub fn total_duration(&self) -> f64 {
        self.segments
            .iter()
            .map(|segment| segment.end_time())
            .fold(0.0, f64::max)
    }
    
    pub fn add_segment(&mut self, segment: TimelineSegment) -> String {
        let id = segment.id.clone();
        
        // Insert segment in chronological order
        let insert_index = self.segments
            .binary_search_by(|s| s.start_time.partial_cmp(&segment.start_time).unwrap())
            .unwrap_or_else(|i| i);
        
        self.segments.insert(insert_index, segment);
        id
    }
    
    pub fn remove_segment(&mut self, segment_id: &str) -> Option<TimelineSegment> {
        if let Some(index) = self.segments.iter().position(|s| s.id == segment_id) {
            Some(self.segments.remove(index))
        } else {
            None
        }
    }
    
    pub fn get_segment(&self, segment_id: &str) -> Option<&TimelineSegment> {
        self.segments.iter().find(|s| s.id == segment_id)
    }
    
    pub fn get_segment_mut(&mut self, segment_id: &str) -> Option<&mut TimelineSegment> {
        self.segments.iter_mut().find(|s| s.id == segment_id)
    }
    
    pub fn get_current_segment(&self) -> Option<&TimelineSegment> {
        self.segments
            .iter()
            .find(|segment| segment.contains_time(self.current_position))
    }
    
    pub fn move_segment(&mut self, segment_id: &str, new_start_time: f64) -> bool {
        if let Some(segment) = self.get_segment_mut(segment_id) {
            segment.start_time = new_start_time.max(0.0);
            
            // Re-sort segments by start time
            self.segments.sort_by(|a, b| a.start_time.partial_cmp(&b.start_time).unwrap());
            true
        } else {
            false
        }
    }
    
    pub fn duplicate_segment(&mut self, segment_id: &str, new_start_time: f64) -> Option<String> {
        if let Some(original) = self.get_segment(segment_id) {
            let mut new_segment = original.clone();
            new_segment.id = generate_segment_id();
            new_segment.start_time = new_start_time.max(0.0);
            
            let new_id = new_segment.id.clone();
            self.add_segment(new_segment);
            Some(new_id)
        } else {
            None
        }
    }
    
    pub fn split_segment(&mut self, segment_id: &str, split_time: f64) -> Option<String> {
        if let Some(original) = self.get_segment(segment_id) {
            if split_time <= original.start_time || split_time >= original.end_time() {
                return None; // Invalid split time
            }
            
            // Calculate how to split the loop count
            let original_duration = original.duration;
            let first_duration = split_time - original.start_time;
            let second_duration = original.end_time() - split_time;
            
            let split_ratio = first_duration / original_duration;
            let first_loop_count = ((original.loop_count as f64 * split_ratio).round() as usize).max(1);
            let second_loop_count = ((original.loop_count as f64 * (1.0 - split_ratio)).round() as usize).max(1);
            
            // Create second segment
            let mut second_segment = original.clone();
            second_segment.id = generate_segment_id();
            second_segment.start_time = split_time;
            second_segment.loop_count = second_loop_count;
            second_segment.duration = second_duration; // Use calculated duration instead of recalculating
            
            let second_id = second_segment.id.clone();
            
            // Update first segment
            if let Some(first_segment) = self.get_segment_mut(segment_id) {
                first_segment.loop_count = first_loop_count;
                first_segment.duration = first_duration; // Use calculated duration instead of recalculating
            }
            
            self.add_segment(second_segment);
            Some(second_id)
        } else {
            None
        }
    }
    
    pub fn play(&mut self) {
        self.playback_state = PlaybackState::Playing;
    }
    
    pub fn pause(&mut self) {
        self.playback_state = PlaybackState::Paused;
    }
    
    pub fn stop(&mut self) {
        self.playback_state = PlaybackState::Stopped;
        self.current_position = 0.0;
    }
    
    pub fn seek(&mut self, position: f64) {
        self.current_position = position.max(0.0).min(self.total_duration());
    }
    
    pub fn advance_position(&mut self, delta_time: f64) -> bool {
        if self.playback_state == PlaybackState::Playing {
            self.current_position += delta_time;
            
            // Check if we've reached the end
            if self.current_position >= self.total_duration() {
                self.stop();
                return false;
            }
            true
        } else {
            false
        }
    }
    
    pub fn is_playing(&self) -> bool {
        self.playback_state == PlaybackState::Playing
    }
    
    pub fn is_empty(&self) -> bool {
        self.segments.is_empty()
    }
}

impl Default for Timeline {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timeline_segment_creation() {
        let ts = TimeSignature::four_four();
        let pattern = Pattern::new("kick_pattern".to_string(), "kick".to_string(), 16);
        let patterns = vec![pattern];
        let segment = TimelineSegment::new(
            "kick_pattern".to_string(),
            patterns,
            0.0,
            4, // 4 loops
            ts,
            120.0, // BPM
        );

        assert_eq!(segment.pattern_id, "kick_pattern");
        assert_eq!(segment.start_time, 0.0);
        assert_eq!(segment.loop_count, 4);
        assert_eq!(segment.time_signature, ts);
        assert_eq!(segment.bpm, 120.0);
        
        // Duration = (4 beats per loop * 4 loops) / (120 BPM / 60) = 16 / 2 = 8 seconds
        assert!((segment.duration - 8.0).abs() < 0.001);
        assert!((segment.end_time() - 8.0).abs() < 0.001);
    }

    #[test]
    fn test_timeline_segment_contains_time() {
        let pattern = Pattern::new("test".to_string(), "kick".to_string(), 16);
        let patterns = vec![pattern];
        let segment = TimelineSegment::new(
            "test".to_string(),
            patterns,
            5.0,
            2,
            TimeSignature::four_four(),
            120.0,
        );

        assert!(!segment.contains_time(4.9));
        assert!(segment.contains_time(5.0));
        assert!(segment.contains_time(7.0));
        assert!(!segment.contains_time(segment.end_time()));
    }

    #[test]
    fn test_timeline_segment_duration_updates() {
        let pattern = Pattern::new("test".to_string(), "kick".to_string(), 16);
        let patterns = vec![pattern];
        let mut segment = TimelineSegment::new(
            "test".to_string(),
            patterns,
            0.0,
            2,
            TimeSignature::four_four(),
            120.0,
        );

        let original_duration = segment.duration;

        // Changing loop count should update duration
        segment.set_loop_count(4);
        assert!((segment.duration - original_duration * 2.0).abs() < 0.001);

        // Changing BPM should update duration
        segment.set_bpm(240.0);
        assert!((segment.duration - original_duration).abs() < 0.001); // Double BPM = half duration

        // Changing time signature should update duration
        segment.set_time_signature(TimeSignature::three_four());
        let expected_duration = (3.0 * 4.0) / (240.0 / 60.0); // 3 beats * 4 loops / 4 BPS
        assert!((segment.duration - expected_duration).abs() < 0.001);
    }

    #[test]
    fn test_timeline_basic_operations() {
        let mut timeline = Timeline::new();
        
        // Test empty timeline
        assert!(timeline.is_empty());
        assert_eq!(timeline.total_duration(), 0.0);
        assert!(timeline.get_current_segment().is_none());

        // Add first segment
        let pattern1 = Pattern::new("pattern1".to_string(), "kick".to_string(), 16);
        let patterns1 = vec![pattern1];
        let segment1 = TimelineSegment::new(
            "pattern1".to_string(),
            patterns1,
            0.0,
            2,
            TimeSignature::four_four(),
            120.0,
        );
        let id1 = timeline.add_segment(segment1);

        assert!(!timeline.is_empty());
        assert!(timeline.total_duration() > 0.0);

        // Add second segment
        let pattern2 = Pattern::new("pattern2".to_string(), "snare".to_string(), 12);
        let patterns2 = vec![pattern2];
        let segment2 = TimelineSegment::new(
            "pattern2".to_string(),
            patterns2,
            10.0,
            1,
            TimeSignature::three_four(),
            120.0,
        );
        let id2 = timeline.add_segment(segment2);

        assert_eq!(timeline.segments.len(), 2);
        
        // Test segments are ordered by start time
        assert!(timeline.segments[0].start_time <= timeline.segments[1].start_time);

        // Test get operations
        assert!(timeline.get_segment(&id1).is_some());
        assert!(timeline.get_segment(&id2).is_some());
        assert!(timeline.get_segment("nonexistent").is_none());

        // Test current segment detection
        timeline.current_position = 2.0; // Within first segment (0-4 seconds)
        let current = timeline.get_current_segment();
        assert!(current.is_some());
        assert_eq!(current.unwrap().id, id1);

        timeline.current_position = 10.5; // Within second segment (starts at 10.0)
        let current = timeline.get_current_segment();
        assert!(current.is_some());
        assert_eq!(current.unwrap().id, id2);

        // Test remove
        let removed = timeline.remove_segment(&id1);
        assert!(removed.is_some());
        assert_eq!(timeline.segments.len(), 1);
        assert!(timeline.get_segment(&id1).is_none());
    }

    #[test]
    fn test_timeline_segment_manipulation() {
        let mut timeline = Timeline::new();
        
        let pattern = Pattern::new("pattern1".to_string(), "kick".to_string(), 16);
        let patterns = vec![pattern];
        let segment = TimelineSegment::new(
            "pattern1".to_string(),
            patterns,
            0.0,
            2,
            TimeSignature::four_four(),
            120.0,
        );
        let id = timeline.add_segment(segment);

        // Test move segment
        assert!(timeline.move_segment(&id, 5.0));
        let moved_segment = timeline.get_segment(&id).unwrap();
        assert_eq!(moved_segment.start_time, 5.0);

        // Test duplicate segment
        let duplicate_id = timeline.duplicate_segment(&id, 15.0);
        assert!(duplicate_id.is_some());
        assert_eq!(timeline.segments.len(), 2);
        
        let duplicate = timeline.get_segment(&duplicate_id.unwrap()).unwrap();
        assert_eq!(duplicate.start_time, 15.0);
        assert_eq!(duplicate.pattern_id, "pattern1");
        assert_ne!(duplicate.id, id);

        // Test split segment
        let split_id = timeline.split_segment(&id, 6.0);
        assert!(split_id.is_some());
        assert_eq!(timeline.segments.len(), 3); // Original + duplicate + split
        
        let original = timeline.get_segment(&id).unwrap();
        let split = timeline.get_segment(&split_id.unwrap()).unwrap();
        assert!(original.end_time() <= split.start_time);
    }

    #[test]
    fn test_timeline_playback_control() {
        let mut timeline = Timeline::new();
        
        // Add segment
        let pattern = Pattern::new("pattern1".to_string(), "kick".to_string(), 16);
        let patterns = vec![pattern];
        let segment = TimelineSegment::new(
            "pattern1".to_string(),
            patterns,
            0.0,
            4,
            TimeSignature::four_four(),
            120.0,
        );
        timeline.add_segment(segment);

        // Test playback states
        assert!(!timeline.is_playing());
        timeline.play();
        assert!(timeline.is_playing());

        timeline.pause();
        assert!(!timeline.is_playing());

        timeline.stop();
        assert!(!timeline.is_playing());
        assert_eq!(timeline.current_position, 0.0);

        // Test seeking
        timeline.seek(5.0);
        assert_eq!(timeline.current_position, 5.0);

        // Test seeking beyond timeline
        timeline.seek(100.0);
        assert_eq!(timeline.current_position, timeline.total_duration());

        // Test position advancement
        timeline.current_position = 0.0;
        timeline.play();
        
        let advanced = timeline.advance_position(2.0);
        assert!(advanced);
        assert_eq!(timeline.current_position, 2.0);

        // Test advancement beyond end
        timeline.current_position = timeline.total_duration() - 1.0;
        let advanced = timeline.advance_position(2.0);
        assert!(!advanced); // Should return false when timeline ends
        assert!(!timeline.is_playing()); // Should stop automatically
    }

    #[test]
    fn test_timeline_with_different_time_signatures() {
        let mut timeline = Timeline::new();
        
        // 4/4 segment: 4 beats * 2 loops = 8 beats at 120 BPM = 4 seconds
        let pattern1 = Pattern::new("pattern_4_4".to_string(), "kick".to_string(), 16);
        let patterns1 = vec![pattern1];
        let segment1 = TimelineSegment::new(
            "pattern_4_4".to_string(),
            patterns1,
            0.0,
            2,
            TimeSignature::four_four(),
            120.0,
        );
        timeline.add_segment(segment1);

        // 3/4 segment: 3 beats * 3 loops = 9 beats at 120 BPM = 4.5 seconds
        let pattern2 = Pattern::new("pattern_3_4".to_string(), "snare".to_string(), 12);
        let patterns2 = vec![pattern2];
        let segment2 = TimelineSegment::new(
            "pattern_3_4".to_string(),
            patterns2,
            4.0,
            3,
            TimeSignature::three_four(),
            120.0,
        );
        timeline.add_segment(segment2);

        // 5/4 segment: 5 beats * 1 loop = 5 beats at 120 BPM = 2.5 seconds
        let pattern3 = Pattern::new("pattern_5_4".to_string(), "hihat".to_string(), 20);
        let patterns3 = vec![pattern3];
        let segment3 = TimelineSegment::new(
            "pattern_5_4".to_string(),
            patterns3,
            8.5,
            1,
            TimeSignature::five_four(),
            120.0,
        );
        timeline.add_segment(segment3);

        // Test total duration
        let expected_total = 8.5 + 2.5; // Last segment end time
        assert!((timeline.total_duration() - expected_total).abs() < 0.001);

        // Test segment detection at various times
        timeline.current_position = 2.0;
        let current = timeline.get_current_segment().unwrap();
        assert_eq!(current.pattern_id, "pattern_4_4");

        timeline.current_position = 5.0;
        let current = timeline.get_current_segment().unwrap();
        assert_eq!(current.pattern_id, "pattern_3_4");

        timeline.current_position = 9.0;
        let current = timeline.get_current_segment().unwrap();
        assert_eq!(current.pattern_id, "pattern_5_4");
    }
}