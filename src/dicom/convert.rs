use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc;

use super::{CoilType, DicomSession, SeriesType};

/// Message sent from the conversion thread to the UI.
pub enum ConvertMessage {
    Log(String),
    Error(String),
    Done { bids_dir: PathBuf },
}

/// Check if dcm2niix is available on PATH.
pub fn find_dcm2niix() -> Option<PathBuf> {
    let output = Command::new("which").arg("dcm2niix").output().ok()?;
    if output.status.success() {
        let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if path.is_empty() { None } else { Some(PathBuf::from(path)) }
    } else {
        let output = Command::new("where").arg("dcm2niix").output().ok()?;
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout).lines().next()?.trim().to_string();
            if path.is_empty() { None } else { Some(PathBuf::from(path)) }
        } else {
            None
        }
    }
}

/// BIDS suffix for a series type.
fn bids_suffix(series_type: SeriesType) -> &'static str {
    match series_type {
        SeriesType::T1w => "T1w",
        _ => "MEGRE",
    }
}

/// BIDS part label for a series type (if applicable).
fn bids_part(series_type: SeriesType) -> Option<&'static str> {
    match series_type {
        SeriesType::Magnitude => Some("mag"),
        SeriesType::Phase => Some("phase"),
        SeriesType::Real => Some("real"),
        SeriesType::Imaginary => Some("imag"),
        _ => None,
    }
}

/// Parameters for building a BIDS filename.
struct BidsNameParts<'a> {
    sub: &'a str,
    ses: Option<&'a str>,
    acq: &'a str,
    run: u32,
    rec: Option<&'a str>,
    echo: Option<u32>,
    part: Option<&'a str>,
    suffix: &'a str,
    extension: &'a str,
}

/// Build the BIDS filename for a converted file.
/// Order: sub-X[_ses-Y]_acq-Z[_rec-R][_run-N][_echo-N][_part-P]_SUFFIX
fn build_bids_filename(p: &BidsNameParts) -> String {
    let mut name = format!("sub-{}", p.sub);
    if let Some(ses) = p.ses {
        name.push_str(&format!("_ses-{}", ses));
    }
    name.push_str(&format!("_acq-{}", p.acq));
    if let Some(rec) = p.rec {
        name.push_str(&format!("_rec-{}", rec));
    }
    if p.run > 1 {
        name.push_str(&format!("_run-{}", p.run));
    }
    if let Some(e) = p.echo {
        name.push_str(&format!("_echo-{}", e));
    }
    if let Some(pt) = p.part {
        name.push_str(&format!("_part-{}", pt));
    }
    name.push_str(&format!("_{}", p.suffix));
    name.push_str(p.extension);
    name
}

/// Parse dcm2niix suffixes from a temp output filename.
/// dcm2niix appends things like `_e1`, `_e2`, `_ph`, `_e1_ph` to the base name.
/// Returns (echo_number, is_phase).
fn parse_dcm2niix_suffixes(filename: &str, base: &str) -> (Option<u32>, bool) {
    let remainder = filename.strip_prefix(base).unwrap_or("");
    let mut echo: Option<u32> = None;
    let mut is_phase = false;

    for part in remainder.split('_') {
        if part.is_empty() {
            continue;
        }
        if part == "ph" {
            is_phase = true;
        } else if let Some(num_str) = part.strip_prefix('e') {
            if let Ok(n) = num_str.parse::<u32>() {
                echo = Some(n);
            }
        }
    }

    (echo, is_phase)
}

// ─── 4D file detection ───

/// Check if a NIfTI file is 4D by reading the header only.
/// Returns the 4th dimension size if > 1.
pub fn nifti_4d_size(path: &Path) -> Option<usize> {
    use nifti::NiftiObject;
    let obj = nifti::ReaderOptions::new().read_file(path).ok()?;
    let header = obj.header();
    if header.dim[0] >= 4 && header.dim[4] > 1 {
        Some(header.dim[4] as usize)
    } else {
        None
    }
}

// ─── JSON sidecar post-processing ───

/// Convert a .nii.gz path to its .json sidecar path.
pub fn nii_to_json_path(nii_path: &Path) -> PathBuf {
    let s = nii_path.to_string_lossy();
    let base = s.strip_suffix(".nii.gz")
        .or_else(|| s.strip_suffix(".nii"))
        .unwrap_or(&s);
    PathBuf::from(format!("{}.json", base))
}

// ─── GE detection ───

/// Check if a series is from a GE scanner.
fn is_ge_manufacturer(manufacturer: &str) -> bool {
    let m = manufacturer.to_uppercase();
    m.contains("GE") || m.contains("GENERAL ELECTRIC")
}

// ─── Main conversion ───

/// Convert a DICOM session to BIDS format using dcm2niix, streaming
/// log/error messages via the provided channel. Sends `Done` when finished.
pub fn convert_session_streaming(
    session: &DicomSession,
    output_dir: &Path,
    tx: &mpsc::Sender<ConvertMessage>,
) {
    if let Err(e) = fs::create_dir_all(output_dir) {
        let _ = tx.send(ConvertMessage::Error(format!("Failed to create output directory: {}", e)));
        let _ = tx.send(ConvertMessage::Done { bids_dir: output_dir.to_path_buf() });
        return;
    }

    let _ = tx.send(ConvertMessage::Log(format!("Output BIDS directory: {}", output_dir.display())));

    for subject in &session.subjects {
        let sub_label = &subject.patient_id;

        for study in &subject.studies {
            let ses_label = if study.study_date.is_empty() {
                None
            } else {
                Some(study.study_date.replace('-', ""))
            };

            // Check if any acquisition has both combined and uncombined coils
            let has_mixed_coils = study.acquisitions.iter().any(|acq| {
                let has_combined = acq.series.iter().any(|s| s.coil_type == CoilType::Combined);
                let has_uncombined = acq.series.iter().any(|s| s.coil_type == CoilType::Uncombined);
                has_combined && has_uncombined
            });

            for acq in &study.acquisitions {
                for series in &acq.series {
                    if series.series_type == SeriesType::Skip {
                        let _ = tx.send(ConvertMessage::Log(format!("Skipping: {} ({})", series.description, acq.name)));
                        continue;
                    }

                    // Add _rec-uncombined only when both combined and uncombined exist
                    let rec = if has_mixed_coils && series.coil_type == CoilType::Uncombined {
                        Some("uncombined")
                    } else {
                        None
                    };

                    let result = convert_series(
                        series,
                        sub_label,
                        ses_label.as_deref(),
                        &acq.name,
                        acq.run_number,
                        rec,
                        output_dir,
                    );

                    for line in result.log_lines {
                        let _ = tx.send(ConvertMessage::Log(line));
                    }
                    for err in result.errors {
                        let _ = tx.send(ConvertMessage::Error(err));
                    }
                }
            }
        }
    }

    let _ = tx.send(ConvertMessage::Done { bids_dir: output_dir.to_path_buf() });
}

struct SeriesConvertResult {
    log_lines: Vec<String>,
    errors: Vec<String>,
}

fn convert_series(
    series: &super::DicomSeries,
    sub_label: &str,
    ses_label: Option<&str>,
    acq_name: &str,
    run_number: u32,
    rec: Option<&str>,
    output_dir: &Path,
) -> SeriesConvertResult {
    let mut log_lines = Vec::new();
    let mut errors = Vec::new();

    let suffix = bids_suffix(series.series_type);
    let part = bids_part(series.series_type);
    let is_ge = is_ge_manufacturer(&series.manufacturer);

    // Build output subdirectory
    let mut sub_dir = output_dir.join(format!("sub-{}", sub_label));
    if let Some(ses) = ses_label {
        sub_dir = sub_dir.join(format!("ses-{}", ses));
    }
    let anat_dir = sub_dir.join("anat");

    if let Err(e) = fs::create_dir_all(&anat_dir) {
        errors.push(format!("Failed to create directory {}: {}", anat_dir.display(), e));
        return SeriesConvertResult { log_lines, errors };
    }

    // Create temp directory for dcm2niix
    let temp_dir = output_dir.join(".tmp_dcm2niix");
    let _ = fs::remove_dir_all(&temp_dir);
    if let Err(e) = fs::create_dir_all(&temp_dir) {
        errors.push(format!("Failed to create temp directory: {}", e));
        return SeriesConvertResult { log_lines, errors };
    }

    // Copy DICOM files to temp directory
    for (i, src) in series.files.iter().enumerate() {
        let ext = src.extension().and_then(|e| e.to_str()).unwrap_or("dcm");
        let dst = temp_dir.join(format!("{:06}.{}", i, ext));
        if let Err(e) = fs::copy(src, &dst) {
            errors.push(format!("Failed to copy {}: {}", src.display(), e));
        }
    }

    log_lines.push(format!(
        "Converting: {} ({} files)",
        series.description, series.num_files,
    ));

    if is_ge {
        log_lines.push("  Note: GE data detected — phase inversion will be applied".to_string());
    }

    // Run dcm2niix with a temp filename — we'll rename outputs ourselves
    let temp_base = "temp_output";
    let dcm2niix_result = Command::new("dcm2niix")
        .args(["-o", &temp_dir.to_string_lossy()])
        .args(["-f", temp_base])
        .args(["-z", "y"])
        .args(["-m", "o"])
        .arg(&temp_dir)
        .output();

    match dcm2niix_result {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);

            for line in stdout.lines() {
                if !line.trim().is_empty() {
                    log_lines.push(format!("  dcm2niix: {}", line));
                }
            }
            for line in stderr.lines() {
                if !line.trim().is_empty() {
                    log_lines.push(format!("  dcm2niix: {}", line));
                }
            }

            if !output.status.success() {
                errors.push(format!("dcm2niix failed for {}: exit code {:?}", series.description, output.status.code()));
            }
        }
        Err(e) => {
            errors.push(format!("Failed to run dcm2niix: {}. Is it installed?", e));
            let _ = fs::remove_dir_all(&temp_dir);
            return SeriesConvertResult { log_lines, errors };
        }
    }

    // Check for 4D NIfTI files produced by dcm2niix
    let nii_files: Vec<PathBuf> = fs::read_dir(&temp_dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
            name.starts_with(temp_base) && (name.ends_with(".nii.gz") || name.ends_with(".nii"))
        })
        .collect();

    for nii_path in &nii_files {
        if let Some(n_vols) = nifti_4d_size(nii_path) {
            log_lines.push(format!(
                "  Warning: 4D NIfTI detected ({} volumes) — dcm2niix merged echoes. \
                 Re-running with -m n to produce separate files.",
                n_vols
            ));

            // Re-run dcm2niix without merging
            let _ = fs::remove_file(nii_path);
            let json_path = nii_to_json_path(nii_path);
            let _ = fs::remove_file(&json_path);

            let rerun = Command::new("dcm2niix")
                .args(["-o", &temp_dir.to_string_lossy()])
                .args(["-f", temp_base])
                .args(["-z", "y"])
                .args(["-m", "n"]) // don't merge
                .arg(&temp_dir)
                .output();

            if let Ok(output) = rerun {
                for line in String::from_utf8_lossy(&output.stdout).lines() {
                    if !line.trim().is_empty() {
                        log_lines.push(format!("  dcm2niix (rerun): {}", line));
                    }
                }
            }
            break; // only need to rerun once
        }
    }

    // Collect all dcm2niix output files
    let final_outputs: Vec<PathBuf> = fs::read_dir(&temp_dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
            name.starts_with(temp_base)
        })
        .collect();

    // Rename to proper BIDS names and move to anat directory
    for temp_path in &final_outputs {
        let filename = temp_path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        let (stem, extension) = if let Some(s) = filename.strip_suffix(".nii.gz") {
            (s, ".nii.gz")
        } else if let Some(s) = filename.strip_suffix(".json") {
            (s, ".json")
        } else if let Some(s) = filename.strip_suffix(".bval") {
            (s, ".bval")
        } else if let Some(s) = filename.strip_suffix(".bvec") {
            (s, ".bvec")
        } else {
            continue;
        };

        // Parse dcm2niix suffixes
        let (echo, is_phase_file) = parse_dcm2niix_suffixes(stem, temp_base);
        let file_part = if is_phase_file { Some("phase") } else { part };

        let bids_name = build_bids_filename(&BidsNameParts {
            sub: sub_label, ses: ses_label, acq: acq_name, run: run_number,
            rec, echo, part: file_part, suffix, extension,
        });

        let dest = anat_dir.join(&bids_name);
        if let Err(e) = fs::rename(temp_path, &dest) {
            if let Err(e2) = fs::copy(temp_path, &dest) {
                errors.push(format!("Failed to move {} -> {}: {} / {}", filename, bids_name, e, e2));
                continue;
            }
        }

        // Post-process JSON sidecars: ensure EchoTime is in seconds
        if extension == ".json" {
            let Ok(content) = fs::read_to_string(&dest) else { continue };
            let Ok(mut json): Result<serde_json::Value, _> = serde_json::from_str(&content) else { continue };

            // Ensure EchoTime is in seconds (some scanners report in ms)
            if let Some(et) = json.get("EchoTime").and_then(|v| v.as_f64()) {
                if et > 1.0 {
                    // Likely in milliseconds, convert to seconds
                    json["EchoTime"] = serde_json::json!(et / 1000.0);
                    if let Ok(output) = serde_json::to_string_pretty(&json) {
                        let _ = fs::write(&dest, output);
                    }
                }
            }

            // For GE data, update ImageType in JSON if we have Real/Imag → Mag/Phase
            if is_ge && matches!(series.series_type, SeriesType::Real | SeriesType::Imaginary) {
                let new_type = if series.series_type == SeriesType::Real {
                    "MAGNITUDE"
                } else {
                    "PHASE"
                };
                json["ImageType"] = serde_json::json!([new_type]);
                if let Ok(output) = serde_json::to_string_pretty(&json) {
                    let _ = fs::write(&dest, output);
                }
            }
        }

        log_lines.push(format!("  -> {}", bids_name));
    }

    // Clean up temp directory
    let _ = fs::remove_dir_all(&temp_dir);

    SeriesConvertResult { log_lines, errors }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_dcm2niix_suffixes() {
        assert_eq!(parse_dcm2niix_suffixes("temp_output", "temp_output"), (None, false));
        assert_eq!(parse_dcm2niix_suffixes("temp_output_e1", "temp_output"), (Some(1), false));
        assert_eq!(parse_dcm2niix_suffixes("temp_output_e2", "temp_output"), (Some(2), false));
        assert_eq!(parse_dcm2niix_suffixes("temp_output_ph", "temp_output"), (None, true));
        assert_eq!(parse_dcm2niix_suffixes("temp_output_e1_ph", "temp_output"), (Some(1), true));
        assert_eq!(parse_dcm2niix_suffixes("temp_output_e12_ph", "temp_output"), (Some(12), true));
    }

    #[test]
    fn test_build_bids_filename_single_echo() {
        let name = build_bids_filename(&BidsNameParts {
            sub: "01", ses: None, acq: "gre", run: 1, rec: None,
            echo: None, part: Some("mag"), suffix: "T2starw", extension: ".nii.gz",
        });
        assert_eq!(name, "sub-01_acq-gre_part-mag_T2starw.nii.gz");
    }

    #[test]
    fn test_build_bids_filename_multi_echo() {
        let name = build_bids_filename(&BidsNameParts {
            sub: "01", ses: Some("20240314"), acq: "gre", run: 1, rec: None,
            echo: Some(3), part: Some("phase"), suffix: "MEGRE", extension: ".json",
        });
        assert_eq!(name, "sub-01_ses-20240314_acq-gre_echo-3_part-phase_MEGRE.json");
    }

    #[test]
    fn test_build_bids_filename_with_run() {
        let name = build_bids_filename(&BidsNameParts {
            sub: "p025pre", ses: Some("20240314"), acq: "wip925B1mmPAT3eco6", run: 2, rec: None,
            echo: Some(1), part: Some("mag"), suffix: "MEGRE", extension: ".nii.gz",
        });
        assert_eq!(name, "sub-p025pre_ses-20240314_acq-wip925B1mmPAT3eco6_run-2_echo-1_part-mag_MEGRE.nii.gz");
    }

    #[test]
    fn test_build_bids_filename_with_rec() {
        let name = build_bids_filename(&BidsNameParts {
            sub: "01", ses: None, acq: "gre", run: 1, rec: Some("uncombined"),
            echo: Some(1), part: Some("mag"), suffix: "MEGRE", extension: ".nii.gz",
        });
        assert_eq!(name, "sub-01_acq-gre_rec-uncombined_echo-1_part-mag_MEGRE.nii.gz");
    }

    #[test]
    fn test_is_ge() {
        assert!(is_ge_manufacturer("GE MEDICAL SYSTEMS"));
        assert!(is_ge_manufacturer("GENERAL ELECTRIC"));
        assert!(!is_ge_manufacturer("SIEMENS"));
        assert!(!is_ge_manufacturer("Philips"));
    }

    // ─── bids_suffix tests ───

    #[test]
    fn test_bids_suffix_t1w() {
        assert_eq!(bids_suffix(SeriesType::T1w), "T1w");
    }

    #[test]
    fn test_bids_suffix_magnitude() {
        assert_eq!(bids_suffix(SeriesType::Magnitude), "MEGRE");
    }

    #[test]
    fn test_bids_suffix_phase() {
        assert_eq!(bids_suffix(SeriesType::Phase), "MEGRE");
    }

    #[test]
    fn test_bids_suffix_real() {
        assert_eq!(bids_suffix(SeriesType::Real), "MEGRE");
    }

    #[test]
    fn test_bids_suffix_imaginary() {
        assert_eq!(bids_suffix(SeriesType::Imaginary), "MEGRE");
    }

    #[test]
    fn test_bids_suffix_extra() {
        assert_eq!(bids_suffix(SeriesType::Extra), "MEGRE");
    }

    #[test]
    fn test_bids_suffix_skip() {
        assert_eq!(bids_suffix(SeriesType::Skip), "MEGRE");
    }

    // ─── bids_part tests ───

    #[test]
    fn test_bids_part_magnitude() {
        assert_eq!(bids_part(SeriesType::Magnitude), Some("mag"));
    }

    #[test]
    fn test_bids_part_phase() {
        assert_eq!(bids_part(SeriesType::Phase), Some("phase"));
    }

    #[test]
    fn test_bids_part_real() {
        assert_eq!(bids_part(SeriesType::Real), Some("real"));
    }

    #[test]
    fn test_bids_part_imaginary() {
        assert_eq!(bids_part(SeriesType::Imaginary), Some("imag"));
    }

    #[test]
    fn test_bids_part_t1w() {
        assert_eq!(bids_part(SeriesType::T1w), None);
    }

    #[test]
    fn test_bids_part_extra() {
        assert_eq!(bids_part(SeriesType::Extra), None);
    }

    #[test]
    fn test_bids_part_skip() {
        assert_eq!(bids_part(SeriesType::Skip), None);
    }

    // ─── nii_to_json_path tests ───

    #[test]
    fn test_nii_to_json_path_nii_gz() {
        let p = nii_to_json_path(Path::new("/data/sub-01_T1w.nii.gz"));
        assert_eq!(p, PathBuf::from("/data/sub-01_T1w.json"));
    }

    #[test]
    fn test_nii_to_json_path_nii() {
        let p = nii_to_json_path(Path::new("/data/sub-01_T1w.nii"));
        assert_eq!(p, PathBuf::from("/data/sub-01_T1w.json"));
    }

    #[test]
    fn test_nii_to_json_path_other_extension() {
        let p = nii_to_json_path(Path::new("/data/sub-01_T1w.bval"));
        assert_eq!(p, PathBuf::from("/data/sub-01_T1w.bval.json"));
    }

    // ─── nifti_4d_size tests ───

    #[test]
    fn test_nifti_4d_size_3d_returns_none() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let nii_path = dir.path().join("mag.nii");
        crate::testutils::write_magnitude(&nii_path);
        assert_eq!(nifti_4d_size(&nii_path), None);
    }

    // ─── is_ge_manufacturer tests ───

    #[test]
    fn test_is_ge_manufacturer_lowercase() {
        assert!(is_ge_manufacturer("ge medical systems"));
    }

    #[test]
    fn test_is_ge_manufacturer_mixed_case() {
        assert!(is_ge_manufacturer("General Electric"));
    }

    #[test]
    fn test_is_ge_manufacturer_empty() {
        assert!(!is_ge_manufacturer(""));
    }

    #[test]
    fn test_is_ge_manufacturer_other_vendors() {
        assert!(!is_ge_manufacturer("SIEMENS HEALTHINEERS"));
        assert!(!is_ge_manufacturer("Philips Medical"));
        assert!(!is_ge_manufacturer("Canon"));
    }

    // ─── build_bids_filename with T1w suffix and no part ───

    #[test]
    fn test_build_bids_filename_t1w_no_part() {
        let name = build_bids_filename(&BidsNameParts {
            sub: "01", ses: None, acq: "mprage", run: 1, rec: None,
            echo: None, part: None, suffix: "T1w", extension: ".nii.gz",
        });
        assert_eq!(name, "sub-01_acq-mprage_T1w.nii.gz");
    }

    #[test]
    fn test_build_bids_filename_t1w_no_part_with_session() {
        let name = build_bids_filename(&BidsNameParts {
            sub: "02", ses: Some("20240101"), acq: "mprage", run: 1, rec: None,
            echo: None, part: None, suffix: "T1w", extension: ".nii.gz",
        });
        assert_eq!(name, "sub-02_ses-20240101_acq-mprage_T1w.nii.gz");
    }
}
