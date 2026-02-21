use serde::Deserialize;
use std::path::Path;

use crate::error::QsmxtError;

/// Relevant fields from a BIDS JSON sidecar.
#[derive(Debug, Clone, Deserialize)]
pub struct QsmSidecar {
    #[serde(rename = "EchoTime")]
    pub echo_time: f64,

    #[serde(rename = "MagneticFieldStrength")]
    pub magnetic_field_strength: f64,

    #[serde(rename = "B0_dir")]
    pub b0_dir: Option<Vec<f64>>,
}

/// Read a BIDS JSON sidecar, extracting QSM-relevant fields.
pub fn read_sidecar(path: &Path) -> crate::Result<QsmSidecar> {
    let text = std::fs::read_to_string(path)?;

    // Parse as generic Value first to give better error messages
    let value: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| QsmxtError::SidecarParse {
            path: path.to_owned(),
            source: e,
        })?;

    let echo_time = value
        .get("EchoTime")
        .and_then(|v| v.as_f64())
        .ok_or_else(|| QsmxtError::MissingSidecarField {
            field: "EchoTime".to_string(),
            path: path.to_owned(),
        })?;

    let magnetic_field_strength = value
        .get("MagneticFieldStrength")
        .and_then(|v| v.as_f64())
        .ok_or_else(|| QsmxtError::MissingSidecarField {
            field: "MagneticFieldStrength".to_string(),
            path: path.to_owned(),
        })?;

    let b0_dir = value.get("B0_dir").and_then(|v| {
        v.as_array().map(|arr| {
            arr.iter()
                .filter_map(|x| x.as_f64())
                .collect::<Vec<f64>>()
        })
    });

    Ok(QsmSidecar {
        echo_time,
        magnetic_field_strength,
        b0_dir,
    })
}
