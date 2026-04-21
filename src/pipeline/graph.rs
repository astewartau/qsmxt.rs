use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::bids::entities::AcquisitionKey;
use crate::error::QsmxtError;
use crate::pipeline::config::PipelineConfig;

/// Metadata about the run, extracted during the load step.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunMetadata {
    pub dims: (usize, usize, usize),
    pub voxel_size: (f64, f64, f64),
    pub affine: [f64; 16],
    pub n_echoes: usize,
    pub echo_times: Vec<f64>,
    pub b0_direction: (f64, f64, f64),
    pub field_strength: f64,
    pub has_magnitude: bool,
}

/// Record of a completed step.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepRecord {
    pub outputs: Vec<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

/// Persistent pipeline state, serialised to JSON.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineState {
    pub version: String,
    pub config_hash: String,
    pub run_key: String,
    pub status: String,
    #[serde(default)]
    pub current_step: Option<String>,
    pub completed_steps: HashMap<String, StepRecord>,
    #[serde(default)]
    pub run_metadata: Option<RunMetadata>,
}

impl PipelineState {
    /// Create a fresh state for a new run.
    pub fn new(config: &PipelineConfig, run_key: &AcquisitionKey) -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION").to_string(),
            config_hash: config_hash(config),
            run_key: format!("{}", run_key),
            status: "pending".to_string(),
            current_step: None,
            completed_steps: HashMap::new(),
            run_metadata: None,
        }
    }

    /// Load existing state from disk, or create new if missing/incompatible.
    pub fn load_or_create(
        state_path: &Path,
        config: &PipelineConfig,
        run_key: &AcquisitionKey,
        force: bool,
    ) -> Self {
        if force {
            return Self::new(config, run_key);
        }

        if let Ok(text) = std::fs::read_to_string(state_path) {
            if let Ok(state) = serde_json::from_str::<PipelineState>(&text) {
                let expected_hash = config_hash(config);
                if state.config_hash == expected_hash {
                    log::info!("Resuming from cached pipeline state");
                    return state;
                } else {
                    log::warn!(
                        "Pipeline config changed (hash mismatch). Re-running from scratch."
                    );
                }
            } else {
                log::warn!("Could not parse pipeline state file. Starting fresh.");
            }
        }

        Self::new(config, run_key)
    }

    /// Save state to disk.
    pub fn save(&self, state_path: &Path) -> crate::Result<()> {
        if let Some(parent) = state_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| QsmxtError::Config(format!("Failed to serialize state: {}", e)))?;
        std::fs::write(state_path, json)?;
        Ok(())
    }

    /// Check if a step is completed and its outputs still exist.
    pub fn is_step_cached(&self, step_name: &str) -> bool {
        if let Some(record) = self.completed_steps.get(step_name) {
            // Verify all output files still exist
            record.outputs.iter().all(|p| p.exists())
        } else {
            false
        }
    }

    /// Mark a step as the current one being processed.
    pub fn set_current(&mut self, step_name: &str) {
        self.status = "in_progress".to_string();
        self.current_step = Some(step_name.to_string());
    }

    /// Mark a step as completed with its output paths.
    pub fn mark_completed(&mut self, step_name: &str, outputs: Vec<PathBuf>) {
        self.completed_steps.insert(
            step_name.to_string(),
            StepRecord {
                outputs,
                metadata: None,
            },
        );
        self.current_step = None;
    }

    /// Mark a step as completed with metadata (e.g., load step stores dims/echo_times).
    pub fn mark_completed_with_metadata(
        &mut self,
        step_name: &str,
        outputs: Vec<PathBuf>,
        metadata: serde_json::Value,
    ) {
        self.completed_steps.insert(
            step_name.to_string(),
            StepRecord {
                outputs,
                metadata: Some(metadata),
            },
        );
        self.current_step = None;
    }

    /// Mark the entire run as complete.
    pub fn mark_run_complete(&mut self) {
        self.status = "complete".to_string();
        self.current_step = None;
    }

    /// Get output paths for a completed step.
    pub fn step_outputs(&self, step_name: &str) -> Option<&[PathBuf]> {
        self.completed_steps
            .get(step_name)
            .map(|r| r.outputs.as_slice())
    }

    /// Invalidate a step and all steps that depend on it.
    pub fn invalidate(&mut self, step_name: &str) {
        self.completed_steps.remove(step_name);
        // Also invalidate downstream steps
        let downstream = downstream_steps(step_name);
        for ds in downstream {
            self.completed_steps.remove(*ds);
        }
    }

    /// Get all completed step names.
    pub fn completed_step_names(&self) -> HashSet<&str> {
        self.completed_steps.keys().map(|s| s.as_str()).collect()
    }
}

/// Compute a hash of the pipeline config for change detection.
fn config_hash(config: &PipelineConfig) -> String {
    let toml = config.to_annotated_toml();
    format!("{:x}", md5_simple(&toml))
}

/// Simple hash (not cryptographic, just for change detection).
fn md5_simple(s: &str) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for b in s.bytes() {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

/// Return step names that depend on the given step (for invalidation).
fn downstream_steps(step_name: &str) -> &'static [&'static str] {
    match step_name {
        "load" => &[
            "resample",
            "scale_phase",
            "inhomog",
            "mask",
            "swi",
            "t2star_r2star",
            "unwrap",
            "bgremove",
            "invert",
            "tgv",
            "reference",
        ],
        "resample" => &[
            "scale_phase",
            "inhomog",
            "mask",
            "swi",
            "t2star_r2star",
            "unwrap",
            "bgremove",
            "invert",
            "tgv",
            "reference",
        ],
        "scale_phase" => &[
            "mask",
            "swi",
            "unwrap",
            "bgremove",
            "invert",
            "tgv",
            "reference",
        ],
        "inhomog" => &[
            "mask",
            "swi",
            "unwrap",
            "bgremove",
            "invert",
            "tgv",
            "reference",
        ],
        "mask" => &[
            "swi",
            "t2star_r2star",
            "unwrap",
            "bgremove",
            "invert",
            "tgv",
            "reference",
        ],
        "swi" => &[],
        "t2star_r2star" => &[],
        "unwrap" => &["bgremove", "invert", "reference"],
        "bgremove" => &["invert", "reference"],
        "invert" | "tgv" => &["reference"],
        "reference" => &[],
        _ => &[],
    }
}

/// The path to the pipeline state file for a given run.
pub fn state_file_path(output_dir: &Path, key: &AcquisitionKey) -> PathBuf {
    let mut dir = output_dir.join(format!("sub-{}", key.subject));
    if let Some(ref ses) = key.session {
        dir = dir.join(format!("ses-{}", ses));
    }
    dir.join("anat").join(".pipeline_state.json")
}

/// Intermediate file path helper.
pub fn intermediate_path(
    output_dir: &Path,
    key: &AcquisitionKey,
    suffix: &str,
) -> PathBuf {
    let mut dir = output_dir.join(format!("sub-{}", key.subject));
    if let Some(ref ses) = key.session {
        dir = dir.join(format!("ses-{}", ses));
    }
    dir.join("anat")
        .join(format!("{}_{}", key.basename(), suffix))
}

/// Remove intermediate files, keeping only final outputs.
pub fn clean_intermediates(state: &PipelineState, output_dir: &Path, key: &AcquisitionKey) {
    let final_steps: HashSet<&str> =
        ["mask", "reference", "swi", "t2star_r2star"].iter().copied().collect();

    for (step_name, record) in &state.completed_steps {
        if !final_steps.contains(step_name.as_str()) {
            for path in &record.outputs {
                if path.exists() {
                    log::info!("Cleaning intermediate: {}", path.display());
                    let _ = std::fs::remove_file(path);
                }
            }
        }
    }

    // Remove state file itself
    let sf = state_file_path(output_dir, key);
    let _ = std::fs::remove_file(sf);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli;

    #[test]
    fn test_new_state() {
        let config = PipelineConfig::from_preset(cli::Preset::Gre);
        let key = AcquisitionKey {
            subject: "01".to_string(),
            session: None,
            acquisition: None,
            reconstruction: None,
            inversion: None,
            run: None,
            suffix: "MEGRE".to_string(),
        };
        let state = PipelineState::new(&config, &key);
        assert_eq!(state.status, "pending");
        assert!(state.completed_steps.is_empty());
        assert!(!state.config_hash.is_empty());
    }

    #[test]
    fn test_mark_completed_and_cached() {
        let config = PipelineConfig::from_preset(cli::Preset::Gre);
        let key = AcquisitionKey {
            subject: "01".to_string(),
            session: None,
            acquisition: None,
            reconstruction: None,
            inversion: None,
            run: None,
            suffix: "MEGRE".to_string(),
        };
        let mut state = PipelineState::new(&config, &key);

        // Not cached yet
        assert!(!state.is_step_cached("mask"));

        // Mark completed with no file paths (metadata-only step)
        state.mark_completed("load", vec![]);
        assert!(state.is_step_cached("load"));

        // Mark completed with a file that doesn't exist — should not be cached
        state.mark_completed("mask", vec![PathBuf::from("/nonexistent/mask.nii")]);
        assert!(!state.is_step_cached("mask"));
    }

    #[test]
    fn test_invalidate_downstream() {
        let config = PipelineConfig::from_preset(cli::Preset::Gre);
        let key = AcquisitionKey {
            subject: "01".to_string(),
            session: None,
            acquisition: None,
            reconstruction: None,
            inversion: None,
            run: None,
            suffix: "MEGRE".to_string(),
        };
        let mut state = PipelineState::new(&config, &key);
        state.mark_completed("load", vec![]);
        state.mark_completed("mask", vec![]);
        state.mark_completed("unwrap", vec![]);
        state.mark_completed("bgremove", vec![]);

        // Invalidating mask should also remove unwrap, bgremove
        state.invalidate("mask");
        assert!(state.completed_steps.contains_key("load"));
        assert!(!state.completed_steps.contains_key("mask"));
        assert!(!state.completed_steps.contains_key("unwrap"));
        assert!(!state.completed_steps.contains_key("bgremove"));
    }

    #[test]
    fn test_state_json_roundtrip() {
        let config = PipelineConfig::from_preset(cli::Preset::Gre);
        let key = AcquisitionKey {
            subject: "01".to_string(),
            session: None,
            acquisition: None,
            reconstruction: None,
            inversion: None,
            run: None,
            suffix: "MEGRE".to_string(),
        };
        let mut state = PipelineState::new(&config, &key);
        state.mark_completed("load", vec![]);
        state.mark_completed("mask", vec![PathBuf::from("/tmp/mask.nii")]);

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("state.json");
        state.save(&path).unwrap();

        let loaded = PipelineState::load_or_create(&path, &config, &key, false);
        assert_eq!(loaded.config_hash, state.config_hash);
        assert!(loaded.completed_steps.contains_key("load"));
        assert!(loaded.completed_steps.contains_key("mask"));
    }

    #[test]
    fn test_config_change_invalidates() {
        let config1 = PipelineConfig::from_preset(cli::Preset::Gre);
        let mut config2 = PipelineConfig::from_preset(cli::Preset::Gre);
        config2.qsm_algorithm = crate::pipeline::config::QsmAlgorithm::Tkd;

        let key = AcquisitionKey {
            subject: "01".to_string(),
            session: None,
            acquisition: None,
            reconstruction: None,
            inversion: None,
            run: None,
            suffix: "MEGRE".to_string(),
        };

        let mut state = PipelineState::new(&config1, &key);
        state.mark_completed("load", vec![]);

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("state.json");
        state.save(&path).unwrap();

        // Loading with different config should start fresh
        let loaded = PipelineState::load_or_create(&path, &config2, &key, false);
        assert!(loaded.completed_steps.is_empty());
    }

    #[test]
    fn test_force_ignores_cache() {
        let config = PipelineConfig::from_preset(cli::Preset::Gre);
        let key = AcquisitionKey {
            subject: "01".to_string(),
            session: None,
            acquisition: None,
            reconstruction: None,
            inversion: None,
            run: None,
            suffix: "MEGRE".to_string(),
        };

        let mut state = PipelineState::new(&config, &key);
        state.mark_completed("load", vec![]);

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("state.json");
        state.save(&path).unwrap();

        let loaded = PipelineState::load_or_create(&path, &config, &key, true);
        assert!(loaded.completed_steps.is_empty());
    }
}
