use std::collections::HashMap;
use std::path::{Path, PathBuf};

use glob::glob;
use log::{debug, warn};

use crate::bids::entities::{self, AcquisitionKey, BidsEntities, Part};
use crate::bids::sidecar;
use crate::error::QsmxtError;

/// Files for a single echo in a BIDS acquisition.
#[derive(Debug, Clone)]
pub struct EchoFiles {
    pub echo_number: u32,
    pub phase_nifti: PathBuf,
    pub phase_json: PathBuf,
    pub magnitude_nifti: Option<PathBuf>,
    pub magnitude_json: Option<PathBuf>,
}

/// A complete QSM acquisition run with all echoes.
#[derive(Debug, Clone)]
pub struct QsmRun {
    pub key: AcquisitionKey,
    pub echoes: Vec<EchoFiles>,
    pub magnetic_field_strength: f64,
    pub echo_times: Vec<f64>,
    pub b0_dir: (f64, f64, f64),
    /// Volume dimensions (nx, ny, nz) from the first phase NIfTI header.
    pub dims: (usize, usize, usize),
    /// Whether magnitude files are available for this run.
    pub has_magnitude: bool,
}

/// Filters for BIDS discovery.
#[derive(Debug, Clone, Default)]
pub struct DiscoveryFilter {
    pub subjects: Option<Vec<String>>,
    pub sessions: Option<Vec<String>>,
    pub acquisitions: Option<Vec<String>>,
    pub runs: Option<Vec<String>>,
    pub num_echoes: Option<usize>,
}

/// Discover all QSM runs in a BIDS directory.
pub fn discover_runs(bids_dir: &Path, filter: &DiscoveryFilter) -> crate::Result<Vec<QsmRun>> {
    let patterns = [
        format!("{}/sub-*/anat/*_part-phase_*.nii*", bids_dir.display()),
        format!(
            "{}/sub-*/ses-*/anat/*_part-phase_*.nii*",
            bids_dir.display()
        ),
    ];

    // Collect all phase files
    let mut phase_files: Vec<(PathBuf, BidsEntities)> = Vec::new();

    for pattern in &patterns {
        for entry in glob(pattern).map_err(|e| QsmxtError::BidsDiscovery(e.to_string()))? {
            let path = entry.map_err(|e| QsmxtError::BidsDiscovery(e.to_string()))?;
            let filename = path
                .file_name()
                .and_then(|f| f.to_str())
                .ok_or_else(|| QsmxtError::BidsDiscovery("Invalid filename".to_string()))?;

            if let Some(ent) = entities::parse_entities(filename) {
                if ent.part != Some(Part::Phase) {
                    continue;
                }

                // Apply filters
                if let Some(ref subs) = filter.subjects {
                    if !subs.iter().any(|s| {
                        s == &ent.subject
                            || s == &format!("sub-{}", ent.subject)
                    }) {
                        continue;
                    }
                }
                if let Some(ref sess) = filter.sessions {
                    match &ent.session {
                        Some(ses) if sess.iter().any(|s| {
                            s == ses || s == &format!("ses-{}", ses)
                        }) => {}
                        Some(_) => continue,
                        None => continue,
                    }
                }
                if let Some(ref acqs) = filter.acquisitions {
                    match &ent.acquisition {
                        Some(acq) if acqs.contains(acq) => {}
                        Some(_) => continue,
                        None if acqs.is_empty() => {}
                        None => continue,
                    }
                }
                if let Some(ref runs) = filter.runs {
                    match &ent.run {
                        Some(run) if runs.contains(run) => {}
                        Some(_) => continue,
                        None => continue,
                    }
                }

                debug!("Found phase file: {}", path.display());
                phase_files.push((path, ent));
            }
        }
    }

    // Group by AcquisitionKey
    let mut groups: HashMap<AcquisitionKey, Vec<(PathBuf, BidsEntities)>> = HashMap::new();
    for (path, ent) in phase_files {
        let key = ent.acquisition_key();
        groups.entry(key).or_default().push((path, ent));
    }

    // Build QsmRun for each group
    let mut runs: Vec<QsmRun> = Vec::new();

    for (key, mut files) in groups {
        // Sort by echo number
        files.sort_by_key(|(_, ent)| ent.echo.unwrap_or(1));

        // Apply echo limit
        if let Some(max_echoes) = filter.num_echoes {
            files.truncate(max_echoes);
        }

        let mut echoes = Vec::new();
        let mut echo_times = Vec::new();
        let mut b0_tesla = 0.0f64;
        let mut b0_dir = (0.0, 0.0, 1.0);

        for (phase_path, ent) in &files {
            let echo_num = ent.echo.unwrap_or(1);

            // Find corresponding files
            let json_path = entities::sidecar_path(phase_path);
            let mag_path = entities::phase_to_magnitude_path(phase_path);

            // Read sidecar
            if !json_path.exists() {
                return Err(QsmxtError::BidsDiscovery(format!(
                    "JSON sidecar not found: {}",
                    json_path.display()
                )));
            }
            let sc = sidecar::read_sidecar(&json_path)?;
            echo_times.push(sc.echo_time);
            b0_tesla = sc.magnetic_field_strength;

            if let Some(ref dir) = sc.b0_dir {
                if dir.len() == 3 {
                    b0_dir = (dir[0], dir[1], dir[2]);
                }
            }

            let mag_nifti = if mag_path.exists() {
                Some(mag_path.clone())
            } else {
                warn!(
                    "Magnitude file not found (will proceed without): {}",
                    mag_path.display()
                );
                None
            };

            let mag_json = mag_nifti.as_ref().map(|p| entities::sidecar_path(p));

            echoes.push(EchoFiles {
                echo_number: echo_num,
                phase_nifti: phase_path.clone(),
                phase_json: json_path,
                magnitude_nifti: mag_nifti,
                magnitude_json: mag_json,
            });
        }

        if echoes.is_empty() {
            continue;
        }

        // Read volume dimensions from the first phase NIfTI header (fast, header-only)
        let dims = qsm_core::nifti_io::read_nifti_dims(&echoes[0].phase_nifti)
            .map_err(|e| QsmxtError::NiftiIo(e))?;
        let has_magnitude = echoes[0].magnitude_nifti.is_some();

        runs.push(QsmRun {
            key,
            echoes,
            magnetic_field_strength: b0_tesla,
            echo_times,
            b0_dir,
            dims,
            has_magnitude,
        });
    }

    // Sort by key for deterministic ordering
    runs.sort_by(|a, b| a.key.to_string().cmp(&b.key.to_string()));

    Ok(runs)
}
