use crate::settings::DefaultSettings;
use crate::timeline::Timeline;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Project metadata and version information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectMetadata {
    pub name: String,
    pub version: String,
    pub created_at: String,
    pub modified_at: String,
    pub author: Option<String>,
    pub description: Option<String>,
}

impl Default for ProjectMetadata {
    fn default() -> Self {
        let now = chrono::Utc::now()
            .format("%Y-%m-%d %H:%M:%S UTC")
            .to_string();
        ProjectMetadata {
            name: "Untitled Project".to_string(),
            version: "1.0".to_string(),
            created_at: now.clone(),
            modified_at: now,
            author: None,
            description: None,
        }
    }
}

/// Main project structure containing all project data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub metadata: ProjectMetadata,
    pub timeline: Timeline,
    pub global_bpm: f32,
    pub global_volume: f32,
}

impl Default for Project {
    fn default() -> Self {
        Project {
            metadata: ProjectMetadata::default(),
            timeline: Timeline::new(),
            global_bpm: 120.0,
            global_volume: 1.0,
        }
    }
}

impl Project {
    /// Create a new empty project
    pub fn new(name: String) -> Self {
        let mut project = Project::default();
        project.metadata.name = name;
        project
    }

    /// Create a new project with default settings applied
    pub fn new_with_defaults(name: String, defaults: &DefaultSettings) -> Self {
        let mut project = Project::default();
        project.metadata.name = name;
        project.global_bpm = defaults.default_bpm;
        project
    }

    /// Save project to a JSON file
    pub fn save_to_file<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        // Update modified timestamp
        self.metadata.modified_at = chrono::Utc::now()
            .format("%Y-%m-%d %H:%M:%S UTC")
            .to_string();

        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    /// Load project from a JSON file
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let project: Project = serde_json::from_str(&content)?;
        Ok(project)
    }

    /// Get the project file extension
    pub fn file_extension() -> &'static str {
        "beatr"
    }

    /// Check if a file is a valid project file based on extension
    pub fn is_project_file<P: AsRef<Path>>(path: P) -> bool {
        path.as_ref()
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.eq_ignore_ascii_case(Self::file_extension()))
            .unwrap_or(false)
    }

    /// Update project metadata
    pub fn update_metadata(
        &mut self,
        name: Option<String>,
        author: Option<String>,
        description: Option<String>,
    ) {
        if let Some(name) = name {
            self.metadata.name = name;
        }
        if let Some(author) = author {
            self.metadata.author = Some(author);
        }
        if let Some(description) = description {
            self.metadata.description = Some(description);
        }
        self.metadata.modified_at = chrono::Utc::now()
            .format("%Y-%m-%d %H:%M:%S UTC")
            .to_string();
    }

    /// Get the timeline for editing
    pub fn timeline_mut(&mut self) -> &mut Timeline {
        &mut self.timeline
    }

    /// Get the timeline for reading
    pub fn timeline(&self) -> &Timeline {
        &self.timeline
    }

    /// Validate project data integrity
    pub fn validate(&self) -> Result<()> {
        // Basic validation checks
        if self.metadata.name.trim().is_empty() {
            return Err(anyhow::anyhow!("Project name cannot be empty"));
        }

        if self.global_bpm < 60.0 || self.global_bpm > 300.0 {
            return Err(anyhow::anyhow!("Global BPM must be between 60 and 300"));
        }

        if self.global_volume < 0.0 || self.global_volume > 2.0 {
            return Err(anyhow::anyhow!("Global volume must be between 0.0 and 2.0"));
        }

        // Validate timeline
        for segment in &self.timeline.segments {
            if segment.start_time < 0.0 {
                return Err(anyhow::anyhow!("Segment start time cannot be negative"));
            }
            if segment.bpm < 60.0 || segment.bpm > 300.0 {
                return Err(anyhow::anyhow!("Segment BPM must be between 60 and 300"));
            }
            if segment.loop_count == 0 {
                return Err(anyhow::anyhow!("Segment loop count must be at least 1"));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_project_creation() {
        let project = Project::new("Test Project".to_string());
        assert_eq!(project.metadata.name, "Test Project");
        assert_eq!(project.global_bpm, 120.0);
        assert_eq!(project.global_volume, 1.0);
        assert_eq!(project.timeline.segments.len(), 0);
    }

    #[test]
    fn test_project_creation_with_defaults() {
        use crate::settings::DefaultSettings;

        let custom_defaults = DefaultSettings {
            default_bpm: 140.0,
            default_time_signature: (3, 4),
            default_pattern_length: 32,
        };

        let project = Project::new_with_defaults("Custom Project".to_string(), &custom_defaults);
        assert_eq!(project.metadata.name, "Custom Project");
        assert_eq!(project.global_bpm, 140.0);
        assert_eq!(project.global_volume, 1.0);
        assert_eq!(project.timeline.segments.len(), 0);

        // Test with default DefaultSettings
        let default_defaults = DefaultSettings::default();
        let default_project =
            Project::new_with_defaults("Default Project".to_string(), &default_defaults);
        assert_eq!(default_project.global_bpm, 120.0);

        // Test validation still passes with custom defaults
        assert!(project.validate().is_ok());
        assert!(default_project.validate().is_ok());
    }

    #[test]
    fn test_project_save_load() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test_project.beatr");

        // Create and save project
        let mut original_project = Project::new("Test Save Load".to_string());
        original_project.global_bpm = 140.0;
        original_project.metadata.author = Some("Test Author".to_string());

        original_project.save_to_file(&file_path).unwrap();

        // Load project
        let loaded_project = Project::load_from_file(&file_path).unwrap();

        // Verify data integrity
        assert_eq!(loaded_project.metadata.name, "Test Save Load");
        assert_eq!(loaded_project.global_bpm, 140.0);
        assert_eq!(
            loaded_project.metadata.author,
            Some("Test Author".to_string())
        );
    }

    #[test]
    fn test_project_validation() {
        let mut project = Project::new("Valid Project".to_string());
        assert!(project.validate().is_ok());

        // Test invalid BPM
        project.global_bpm = 500.0;
        assert!(project.validate().is_err());

        // Reset and test invalid volume
        project.global_bpm = 120.0;
        project.global_volume = -1.0;
        assert!(project.validate().is_err());

        // Reset and test empty name
        project.global_volume = 1.0;
        project.metadata.name = "".to_string();
        assert!(project.validate().is_err());
    }

    #[test]
    fn test_file_extension_detection() {
        assert!(Project::is_project_file("test.beatr"));
        assert!(Project::is_project_file("test.BEATR"));
        assert!(!Project::is_project_file("test.txt"));
        assert!(!Project::is_project_file("test"));
    }

    #[test]
    fn test_project_with_timeline_segments() {
        use crate::audio::{sequencer::Pattern, TimeSignature};
        use crate::timeline::TimelineSegment;

        let dir = tempdir().unwrap();
        let file_path = dir.path().join("timeline_test.beatr");

        // Create project with timeline segments
        let mut project = Project::new("Timeline Test".to_string());

        // Create patterns
        let mut kick_pattern = Pattern::new("Kick".to_string(), "kick".to_string(), 16);
        kick_pattern.steps[0].active = true;
        kick_pattern.steps[4].active = true;
        kick_pattern.steps[8].active = true;
        kick_pattern.steps[12].active = true;

        let mut snare_pattern = Pattern::new("Snare".to_string(), "snare".to_string(), 16);
        snare_pattern.steps[4].active = true;
        snare_pattern.steps[12].active = true;

        let patterns = vec![kick_pattern, snare_pattern];

        // Create timeline segment
        let segment = TimelineSegment::new(
            "Test Pattern".to_string(),
            patterns,
            0.0,
            2, // 2 loops
            TimeSignature::four_four(),
            130.0, // 130 BPM
        );

        project.timeline.add_segment(segment);

        // Save project
        project.save_to_file(&file_path).unwrap();

        // Load project
        let loaded_project = Project::load_from_file(&file_path).unwrap();

        // Verify timeline data integrity
        assert_eq!(loaded_project.timeline.segments.len(), 1);

        let loaded_segment = &loaded_project.timeline.segments[0];
        assert_eq!(loaded_segment.pattern_id, "Test Pattern");
        assert_eq!(loaded_segment.bpm, 130.0);
        assert_eq!(loaded_segment.loop_count, 2);
        assert_eq!(loaded_segment.patterns.len(), 2);

        // Verify pattern data
        let loaded_kick = &loaded_segment.patterns[0];
        assert_eq!(loaded_kick.name, "Kick");
        assert_eq!(loaded_kick.sample_name, "kick");
        assert!(loaded_kick.steps[0].active);
        assert!(loaded_kick.steps[4].active);
        assert!(!loaded_kick.steps[1].active);

        let loaded_snare = &loaded_segment.patterns[1];
        assert_eq!(loaded_snare.name, "Snare");
        assert_eq!(loaded_snare.sample_name, "snare");
        assert!(loaded_snare.steps[4].active);
        assert!(loaded_snare.steps[12].active);
        assert!(!loaded_snare.steps[0].active);

        println!("✅ Timeline serialization integration test passed");
    }

    #[test]
    fn test_project_metadata_serialization() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("metadata_test.beatr");

        let mut project = Project::new("Metadata Test".to_string());
        project.update_metadata(
            Some("Updated Project Name".to_string()),
            Some("Test Author".to_string()),
            Some("Test description for serialization".to_string()),
        );
        project.global_bpm = 140.0;
        project.global_volume = 0.8;

        // Save and load
        project.save_to_file(&file_path).unwrap();
        let loaded_project = Project::load_from_file(&file_path).unwrap();

        // Verify metadata
        assert_eq!(loaded_project.metadata.name, "Updated Project Name");
        assert_eq!(
            loaded_project.metadata.author,
            Some("Test Author".to_string())
        );
        assert_eq!(
            loaded_project.metadata.description,
            Some("Test description for serialization".to_string())
        );
        assert_eq!(loaded_project.global_bpm, 140.0);
        assert_eq!(loaded_project.global_volume, 0.8);

        // Verify timestamps exist
        assert!(!loaded_project.metadata.created_at.is_empty());
        assert!(!loaded_project.metadata.modified_at.is_empty());

        println!("✅ Project metadata serialization test passed");
    }
}
