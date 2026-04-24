use std::path::{Path, PathBuf};

use crate::bids::entities::AcquisitionKey;

/// Manages BIDS derivative output paths.
pub struct DerivativeOutputs {
    pub output_dir: PathBuf,
}

impl DerivativeOutputs {
    pub fn new(output_dir: &Path) -> Self {
        Self {
            output_dir: output_dir.to_owned(),
        }
    }

    /// Build the subject/session anat directory for outputs.
    fn anat_dir(&self, key: &AcquisitionKey) -> PathBuf {
        let mut dir = self.output_dir.join(format!("sub-{}", key.subject));
        if let Some(ref ses) = key.session {
            dir = dir.join(format!("ses-{}", ses));
        }
        dir.join("anat")
    }

    /// Build a NIfTI output path with the given suffix.
    fn nifti_path(&self, key: &AcquisitionKey, suffix: &str) -> PathBuf {
        self.anat_dir(key).join(format!("{}_{}.nii", key.basename(), suffix))
    }

    // Final outputs
    pub fn qsm_path(&self, key: &AcquisitionKey) -> PathBuf { self.nifti_path(key, "Chimap") }
    pub fn mask_path(&self, key: &AcquisitionKey) -> PathBuf { self.nifti_path(key, "mask") }
    pub fn swi_path(&self, key: &AcquisitionKey) -> PathBuf { self.nifti_path(key, "swi") }
    pub fn swi_mip_path(&self, key: &AcquisitionKey) -> PathBuf { self.nifti_path(key, "minIP") }
    pub fn t2star_path(&self, key: &AcquisitionKey) -> PathBuf { self.nifti_path(key, "T2starmap") }
    pub fn r2star_path(&self, key: &AcquisitionKey) -> PathBuf { self.nifti_path(key, "R2starmap") }

    // Intermediate outputs
    pub fn field_ppm_path(&self, key: &AcquisitionKey) -> PathBuf { self.nifti_path(key, "field-ppm") }
    pub fn local_field_path(&self, key: &AcquisitionKey) -> PathBuf { self.nifti_path(key, "localfield") }
    pub fn bg_mask_path(&self, key: &AcquisitionKey) -> PathBuf { self.nifti_path(key, "bgmask") }
    pub fn chi_raw_path(&self, key: &AcquisitionKey) -> PathBuf { self.nifti_path(key, "Chimap-raw") }

    // Per-echo intermediates
    pub fn phase_scaled_path(&self, key: &AcquisitionKey, echo: usize) -> PathBuf {
        self.anat_dir(key).join(format!("{}_echo-{}_phase-scaled.nii", key.basename(), echo))
    }
    pub fn mag_path(&self, key: &AcquisitionKey, echo: usize) -> PathBuf {
        self.anat_dir(key).join(format!("{}_echo-{}_mag.nii", key.basename(), echo))
    }

    // Pipeline state
    pub fn state_path(&self, key: &AcquisitionKey) -> PathBuf {
        self.anat_dir(key).join(".pipeline_state.json")
    }

    /// Write the BIDS dataset_description.json for this derivative.
    pub fn write_dataset_description(&self) -> crate::Result<()> {
        std::fs::create_dir_all(&self.output_dir)?;

        let desc = serde_json::json!({
            "Name": "qsmxt",
            "BIDSVersion": "1.9.0",
            "GeneratedBy": [{
                "Name": "qsmxt.rs",
                "Version": env!("CARGO_PKG_VERSION"),
            }],
            "PipelineDescription": {
                "Name": "QSMxT",
                "Description": "Quantitative Susceptibility Mapping pipeline"
            }
        });

        let path = self.output_dir.join("dataset_description.json");
        let json = serde_json::to_string_pretty(&desc).unwrap();
        std::fs::write(path, json)?;
        Ok(())
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

    #[test]
    fn test_field_ppm_path() {
        let path = output().field_ppm_path(&key_no_session());
        assert!(path.to_str().unwrap().ends_with("_field-ppm.nii"));
    }

    #[test]
    fn test_local_field_path() {
        let path = output().local_field_path(&key_no_session());
        assert!(path.to_str().unwrap().ends_with("_localfield.nii"));
    }

    #[test]
    fn test_bg_mask_path() {
        let path = output().bg_mask_path(&key_no_session());
        assert!(path.to_str().unwrap().ends_with("_bgmask.nii"));
    }

    #[test]
    fn test_chi_raw_path() {
        let path = output().chi_raw_path(&key_no_session());
        assert!(path.to_str().unwrap().ends_with("_Chimap-raw.nii"));
    }

    #[test]
    fn test_phase_scaled_path() {
        let path = output().phase_scaled_path(&key_no_session(), 2);
        let name = path.file_name().unwrap().to_str().unwrap();
        assert!(name.contains("echo-2"));
        assert!(name.contains("phase-scaled"));
    }

    #[test]
    fn test_mag_path() {
        let path = output().mag_path(&key_no_session(), 1);
        let name = path.file_name().unwrap().to_str().unwrap();
        assert!(name.contains("echo-1"));
        assert!(name.contains("mag"));
    }

    #[test]
    fn test_state_path() {
        let path = output().state_path(&key_no_session());
        assert!(path.to_str().unwrap().ends_with(".pipeline_state.json"));
    }

    #[test]
    fn test_write_dataset_description() {
        let dir = tempfile::tempdir().unwrap();
        let o = DerivativeOutputs::new(dir.path());
        o.write_dataset_description().unwrap();
        let desc_path = dir.path().join("dataset_description.json");
        assert!(desc_path.exists());
        let content = std::fs::read_to_string(desc_path).unwrap();
        assert!(content.contains("qsmxt"));
        assert!(content.contains("BIDSVersion"));
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
