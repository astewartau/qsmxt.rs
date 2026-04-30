use std::path::{Path, PathBuf};

use crate::bids::entities::AcquisitionKey;

/// Manages BIDS derivative output paths.
///
/// Final outputs (QSM, mask, magnitude, SWI, T2*, R2*) go to `output_dir/sub-XX/anat/`.
/// Intermediates (per-echo data, field maps, raw chi, pipeline state) go to
/// `output_dir/workflow/sub-XX/anat/`.
pub struct DerivativeOutputs {
    pub output_dir: PathBuf,
}

impl DerivativeOutputs {
    pub fn new(output_dir: &Path) -> Self {
        Self {
            output_dir: output_dir.to_owned(),
        }
    }

    /// Build the subject/session anat directory for final outputs.
    fn anat_dir(&self, key: &AcquisitionKey) -> PathBuf {
        let mut dir = self.output_dir.join(format!("sub-{}", key.subject));
        if let Some(ref ses) = key.session {
            dir = dir.join(format!("ses-{}", ses));
        }
        dir.join("anat")
    }

    /// Build the subject/session anat directory for workflow intermediates.
    fn workflow_anat_dir(&self, key: &AcquisitionKey) -> PathBuf {
        let mut dir = self.output_dir.join("workflow").join(format!("sub-{}", key.subject));
        if let Some(ref ses) = key.session {
            dir = dir.join(format!("ses-{}", ses));
        }
        dir.join("anat")
    }

    /// Build a NIfTI output path with the given suffix (final outputs).
    fn nifti_path(&self, key: &AcquisitionKey, suffix: &str) -> PathBuf {
        self.anat_dir(key).join(format!("{}_{}.nii", key.basename(), suffix))
    }

    /// Build a NIfTI output path with the given suffix (workflow intermediates).
    fn workflow_nifti_path(&self, key: &AcquisitionKey, suffix: &str) -> PathBuf {
        self.workflow_anat_dir(key).join(format!("{}_{}.nii", key.basename(), suffix))
    }

    // Final outputs
    pub fn qsm_path(&self, key: &AcquisitionKey) -> PathBuf { self.nifti_path(key, "Chimap") }
    pub fn mask_path(&self, key: &AcquisitionKey) -> PathBuf { self.nifti_path(key, "mask") }
    pub fn magnitude_path(&self, key: &AcquisitionKey) -> PathBuf { self.nifti_path(key, "magnitude") }
    pub fn swi_path(&self, key: &AcquisitionKey) -> PathBuf { self.nifti_path(key, "swi") }
    pub fn swi_mip_path(&self, key: &AcquisitionKey) -> PathBuf { self.nifti_path(key, "minIP") }
    pub fn t2star_path(&self, key: &AcquisitionKey) -> PathBuf { self.nifti_path(key, "T2starmap") }
    pub fn r2star_path(&self, key: &AcquisitionKey) -> PathBuf { self.nifti_path(key, "R2starmap") }

    // Intermediate outputs (workflow)
    pub fn field_ppm_path(&self, key: &AcquisitionKey) -> PathBuf { self.workflow_nifti_path(key, "field-ppm") }
    pub fn local_field_path(&self, key: &AcquisitionKey) -> PathBuf { self.workflow_nifti_path(key, "localfield") }
    pub fn bg_mask_path(&self, key: &AcquisitionKey) -> PathBuf { self.workflow_nifti_path(key, "bgmask") }
    pub fn chi_raw_path(&self, key: &AcquisitionKey) -> PathBuf { self.workflow_nifti_path(key, "Chimap-raw") }

    // Per-echo intermediates (workflow)
    pub fn phase_scaled_path(&self, key: &AcquisitionKey, echo: usize) -> PathBuf {
        self.workflow_anat_dir(key).join(format!("{}_echo-{}_phase-scaled.nii", key.basename(), echo))
    }
    pub fn mag_path(&self, key: &AcquisitionKey, echo: usize) -> PathBuf {
        self.workflow_anat_dir(key).join(format!("{}_echo-{}_mag.nii", key.basename(), echo))
    }

    // Pipeline state (workflow)
    pub fn state_path(&self, key: &AcquisitionKey) -> PathBuf {
        self.workflow_anat_dir(key).join(".pipeline_state.json")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key_no_session() -> AcquisitionKey {
        AcquisitionKey {
            subject: "01".to_string(),
            session: None,
            acquisition: None,
            reconstruction: None,
            inversion: None,
            run: None,
            suffix: "MEGRE".to_string(),
        }
    }

    fn key_with_session() -> AcquisitionKey {
        AcquisitionKey {
            subject: "01".to_string(),
            session: Some("pre".to_string()),
            acquisition: None,
            reconstruction: None,
            inversion: None,
            run: None,
            suffix: "MEGRE".to_string(),
        }
    }

    fn output() -> DerivativeOutputs {
        DerivativeOutputs::new(Path::new("/out"))
    }

    // --- Final output paths ---

    #[test]
    fn test_qsm_path_no_session() {
        let path = output().qsm_path(&key_no_session());
        assert_eq!(path, PathBuf::from("/out/sub-01/anat/sub-01_Chimap.nii"));
    }

    #[test]
    fn test_qsm_path_with_session() {
        let path = output().qsm_path(&key_with_session());
        assert_eq!(path, PathBuf::from("/out/sub-01/ses-pre/anat/sub-01_ses-pre_Chimap.nii"));
    }

    #[test]
    fn test_mask_path() {
        let path = output().mask_path(&key_no_session());
        assert!(path.to_str().unwrap().ends_with("_mask.nii"));
        assert!(!path.to_str().unwrap().contains("workflow"));
    }

    #[test]
    fn test_magnitude_path() {
        let path = output().magnitude_path(&key_no_session());
        assert!(path.to_str().unwrap().ends_with("_magnitude.nii"));
        assert!(!path.to_str().unwrap().contains("workflow"));
    }

    #[test]
    fn test_swi_and_mip_paths() {
        let o = output();
        let key = key_no_session();
        assert!(o.swi_path(&key).to_str().unwrap().ends_with("_swi.nii"));
        assert!(o.swi_mip_path(&key).to_str().unwrap().ends_with("_minIP.nii"));
    }

    #[test]
    fn test_t2star_and_r2star_paths() {
        let o = output();
        let key = key_no_session();
        assert!(o.t2star_path(&key).to_str().unwrap().ends_with("_T2starmap.nii"));
        assert!(o.r2star_path(&key).to_str().unwrap().ends_with("_R2starmap.nii"));
    }

    // --- Workflow (intermediate) paths ---

    #[test]
    fn test_field_ppm_path() {
        let path = output().field_ppm_path(&key_no_session());
        assert!(path.to_str().unwrap().ends_with("_field-ppm.nii"));
        assert!(path.to_str().unwrap().contains("workflow"));
    }

    #[test]
    fn test_local_field_path() {
        let path = output().local_field_path(&key_no_session());
        assert!(path.to_str().unwrap().ends_with("_localfield.nii"));
        assert!(path.to_str().unwrap().contains("workflow"));
    }

    #[test]
    fn test_bg_mask_path() {
        let path = output().bg_mask_path(&key_no_session());
        assert!(path.to_str().unwrap().ends_with("_bgmask.nii"));
        assert!(path.to_str().unwrap().contains("workflow"));
    }

    #[test]
    fn test_chi_raw_path() {
        let path = output().chi_raw_path(&key_no_session());
        assert!(path.to_str().unwrap().ends_with("_Chimap-raw.nii"));
        assert!(path.to_str().unwrap().contains("workflow"));
    }

    #[test]
    fn test_phase_scaled_path() {
        let path = output().phase_scaled_path(&key_no_session(), 2);
        let s = path.to_str().unwrap();
        assert!(s.contains("echo-2"));
        assert!(s.contains("phase-scaled"));
        assert!(s.contains("workflow"));
    }

    #[test]
    fn test_mag_path() {
        let path = output().mag_path(&key_no_session(), 1);
        let s = path.to_str().unwrap();
        assert!(s.contains("echo-1"));
        assert!(s.contains("mag"));
        assert!(s.contains("workflow"));
    }

    #[test]
    fn test_state_path() {
        let path = output().state_path(&key_no_session());
        let s = path.to_str().unwrap();
        assert!(s.ends_with(".pipeline_state.json"));
        assert!(s.contains("workflow"));
    }

    // --- Full path structure ---

    #[test]
    fn test_final_vs_workflow_separation() {
        let o = output();
        let key = key_no_session();
        // Final outputs are NOT under workflow/
        assert_eq!(o.qsm_path(&key), PathBuf::from("/out/sub-01/anat/sub-01_Chimap.nii"));
        // Intermediates ARE under workflow/
        assert_eq!(o.field_ppm_path(&key), PathBuf::from("/out/workflow/sub-01/anat/sub-01_field-ppm.nii"));
        assert_eq!(o.state_path(&key), PathBuf::from("/out/workflow/sub-01/anat/.pipeline_state.json"));
    }

    #[test]
    fn test_workflow_path_with_session() {
        let o = output();
        let key = key_with_session();
        assert_eq!(
            o.field_ppm_path(&key),
            PathBuf::from("/out/workflow/sub-01/ses-pre/anat/sub-01_ses-pre_field-ppm.nii")
        );
    }

    #[test]
    fn test_path_with_all_entities() {
        let key = AcquisitionKey {
            subject: "02".to_string(),
            session: Some("post".to_string()),
            acquisition: Some("gre".to_string()),
            reconstruction: None,
            inversion: None,
            run: Some("1".to_string()),
            suffix: "MEGRE".to_string(),
        };
        let path = output().qsm_path(&key);
        let name = path.file_name().unwrap().to_str().unwrap();
        assert!(name.contains("sub-02"));
        assert!(name.contains("ses-post"));
        assert!(name.contains("acq-gre"));
        assert!(name.contains("run-1"));
        assert!(name.ends_with("_Chimap.nii"));
    }
}
