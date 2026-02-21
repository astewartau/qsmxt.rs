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

    pub fn qsm_path(&self, key: &AcquisitionKey) -> PathBuf {
        self.anat_dir(key)
            .join(format!("{}_Chimap.nii", key.basename()))
    }

    pub fn mask_path(&self, key: &AcquisitionKey) -> PathBuf {
        self.anat_dir(key)
            .join(format!("{}_mask.nii", key.basename()))
    }

    pub fn swi_path(&self, key: &AcquisitionKey) -> PathBuf {
        self.anat_dir(key)
            .join(format!("{}_swi.nii", key.basename()))
    }

    pub fn swi_mip_path(&self, key: &AcquisitionKey) -> PathBuf {
        self.anat_dir(key)
            .join(format!("{}_minIP.nii", key.basename()))
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
