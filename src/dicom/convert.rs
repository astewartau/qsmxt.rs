use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc;

use super::{DicomSession, SeriesType};

/// Message sent from the conversion thread to the UI.
pub enum ConvertMessage {
    Log(String),
    Error(String),
    Done { bids_dir: PathBuf },
}

/// Result of a DICOM → BIDS conversion.
#[derive(Debug)]
#[allow(dead_code)]
pub struct ConvertResult {
    pub bids_dir: PathBuf,
    pub log_lines: Vec<String>,
    pub errors: Vec<String>,
}

/// Check if dcm2niix is available on PATH.
pub fn find_dcm2niix() -> Option<PathBuf> {
    which_dcm2niix()
}

fn which_dcm2niix() -> Option<PathBuf> {
    let output = Command::new("which").arg("dcm2niix").output().ok()?;
    if output.status.success() {
        let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if path.is_empty() { None } else { Some(PathBuf::from(path)) }
    } else {
        // Try Windows-style
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

            for acq in &study.acquisitions {
                for series in &acq.series {
                    if series.series_type == SeriesType::Skip {
                        let _ = tx.send(ConvertMessage::Log(format!("Skipping: {} ({})", series.description, acq.name)));
                        continue;
                    }

                    let result = convert_series(
                        series,
                        sub_label,
                        ses_label.as_deref(),
                        &acq.name,
                        acq.run_number,
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

/// Parse dcm2niix suffixes from a temp output filename.
/// dcm2niix appends things like `_e1`, `_e2`, `_ph`, `_e1_ph` to the base name.
/// Returns (echo_number, is_phase).
fn parse_dcm2niix_suffixes(filename: &str, base: &str) -> (Option<u32>, bool) {
    // Strip the base prefix to get the suffix part
    let remainder = filename.strip_prefix(base).unwrap_or("");
    // remainder might be "_e1", "_e1_ph", "_ph", or ""
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

/// Parameters for building a BIDS filename.
struct BidsNameParts<'a> {
    sub: &'a str,
    ses: Option<&'a str>,
    acq: &'a str,
    run: u32,
    echo: Option<u32>,
    part: Option<&'a str>,
    suffix: &'a str,
    extension: &'a str,
}

/// Build the BIDS filename for a converted file.
/// Order: sub-X[_ses-Y]_acq-Z[_run-N][_echo-N][_part-P]_SUFFIX
fn build_bids_filename(p: &BidsNameParts) -> String {
    let mut name = format!("sub-{}", p.sub);
    if let Some(ses) = p.ses {
        name.push_str(&format!("_ses-{}", ses));
    }
    name.push_str(&format!("_acq-{}", p.acq));
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

fn convert_series(
    series: &super::DicomSeries,
    sub_label: &str,
    ses_label: Option<&str>,
    acq_name: &str,
    run_number: u32,
    output_dir: &Path,
) -> SeriesConvertResult {
    let mut log_lines = Vec::new();
    let mut errors = Vec::new();

    let suffix = bids_suffix(series.series_type);
    let part = bids_part(series.series_type);

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

    // Run dcm2niix with a temp filename — we'll rename outputs ourselves
    let temp_base = "temp_output";
    let dcm2niix_result = Command::new("dcm2niix")
        .args(["-o", &temp_dir.to_string_lossy()])
        .args(["-f", temp_base])
        .args(["-z", "y"])  // gzip compress
        .args(["-m", "o"])  // merge 2D slices into 3D
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

    // Post-process: find dcm2niix outputs and rename to proper BIDS names
    let temp_outputs: Vec<PathBuf> = fs::read_dir(&temp_dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
            name.starts_with(temp_base)
        })
        .collect();

    for temp_path in &temp_outputs {
        let filename = temp_path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        // Determine extension (.nii.gz, .json, .bval, .bvec)
        let (stem, extension) = if let Some(s) = filename.strip_suffix(".nii.gz") {
            (s, ".nii.gz")
        } else if let Some(s) = filename.strip_suffix(".json") {
            (s, ".json")
        } else if let Some(s) = filename.strip_suffix(".bval") {
            (s, ".bval")
        } else if let Some(s) = filename.strip_suffix(".bvec") {
            (s, ".bvec")
        } else {
            continue; // skip unknown files (e.g. the copied DICOMs)
        };

        // Parse dcm2niix suffixes from the stem
        let (echo, is_phase) = parse_dcm2niix_suffixes(stem, temp_base);

        // Determine part label: if dcm2niix flagged as phase (_ph suffix),
        // use "phase"; otherwise use the series-level type
        let file_part = if is_phase { Some("phase") } else { part };

        let bids_name = build_bids_filename(&BidsNameParts {
            sub: sub_label, ses: ses_label, acq: acq_name, run: run_number,
            echo, part: file_part, suffix, extension,
        });

        let dest = anat_dir.join(&bids_name);
        if let Err(e) = fs::rename(temp_path, &dest) {
            // rename may fail across filesystems, fall back to copy
            if let Err(e2) = fs::copy(temp_path, &dest) {
                errors.push(format!("Failed to move {} -> {}: {} / {}", filename, bids_name, e, e2));
                continue;
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
            sub: "01", ses: None, acq: "gre", run: 1,
            echo: None, part: Some("mag"), suffix: "T2starw", extension: ".nii.gz",
        });
        assert_eq!(name, "sub-01_acq-gre_part-mag_T2starw.nii.gz");
    }

    #[test]
    fn test_build_bids_filename_multi_echo() {
        let name = build_bids_filename(&BidsNameParts {
            sub: "01", ses: Some("20240314"), acq: "gre", run: 1,
            echo: Some(3), part: Some("phase"), suffix: "MEGRE", extension: ".json",
        });
        assert_eq!(name, "sub-01_ses-20240314_acq-gre_echo-3_part-phase_MEGRE.json");
    }

    #[test]
    fn test_build_bids_filename_with_run() {
        let name = build_bids_filename(&BidsNameParts {
            sub: "p025pre", ses: Some("20240314"), acq: "wip925B1mmPAT3eco6", run: 2,
            echo: Some(1), part: Some("mag"), suffix: "MEGRE", extension: ".nii.gz",
        });
        assert_eq!(name, "sub-p025pre_ses-20240314_acq-wip925B1mmPAT3eco6_run-2_echo-1_part-mag_MEGRE.nii.gz");
    }
}
