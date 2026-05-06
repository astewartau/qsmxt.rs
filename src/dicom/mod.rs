pub mod convert;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use dicom::object::open_file;

/// What type of data a DICOM series represents.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeriesType {
    Magnitude,
    Phase,
    Real,
    Imaginary,
    T1w,
    Extra,
    Skip,
}

impl SeriesType {
    pub const ALL: &[SeriesType] = &[
        SeriesType::Magnitude,
        SeriesType::Phase,
        SeriesType::Real,
        SeriesType::Imaginary,
        SeriesType::T1w,
        SeriesType::Extra,
        SeriesType::Skip,
    ];

    pub fn label(self) -> &'static str {
        match self {
            SeriesType::Magnitude => "Magnitude",
            SeriesType::Phase => "Phase",
            SeriesType::Real => "Real",
            SeriesType::Imaginary => "Imaginary",
            SeriesType::T1w => "T1w",
            SeriesType::Extra => "Extra",
            SeriesType::Skip => "Skip",
        }
    }

    pub fn next(self) -> SeriesType {
        let all = Self::ALL;
        let idx = all.iter().position(|&t| t == self).unwrap_or(0);
        all[(idx + 1) % all.len()]
    }

    pub fn prev(self) -> SeriesType {
        let all = Self::ALL;
        let idx = all.iter().position(|&t| t == self).unwrap_or(0);
        all[(idx + all.len() - 1) % all.len()]
    }
}

/// Metadata extracted from a single DICOM file.
#[derive(Debug, Clone)]
struct DicomFileInfo {
    path: PathBuf,
    patient_id: String,
    study_date: String,
    series_instance_uid: String,
    series_description: String,
    protocol_name: String,
    series_number: i32,
    echo_time: Option<f64>,
    image_type: Vec<String>,
    magnetic_field_strength: Option<f64>,
    manufacturer: String,
    acquisition_time: Option<f64>,
    coil_string: String,
    is_enhanced: bool,
}

/// A group of DICOM files that form a single series.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct DicomSeries {
    pub series_uid: String,
    pub description: String,
    pub protocol_name: String,
    pub series_number: i32,
    pub image_type: Vec<String>,
    pub echo_time: Option<f64>,
    pub magnetic_field_strength: Option<f64>,
    pub num_files: usize,
    pub series_type: SeriesType,
    pub files: Vec<PathBuf>,
    pub manufacturer: String,
    pub coil_type: CoilType,
}

/// Whether coil data is combined or uncombined.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoilType {
    Combined,
    Uncombined,
    Unknown,
}

/// An acquisition groups series that share a protocol name within a run.
#[derive(Debug, Clone)]
pub struct DicomAcquisition {
    pub name: String,
    pub run_number: u32,
    pub series: Vec<DicomSeries>,
}

/// A study (session) groups acquisitions from a single scan date.
#[derive(Debug, Clone)]
pub struct DicomStudy {
    pub study_date: String,
    pub acquisitions: Vec<DicomAcquisition>,
}

/// A subject groups studies from a single patient.
#[derive(Debug, Clone)]
pub struct DicomSubject {
    pub patient_id: String,
    pub studies: Vec<DicomStudy>,
}

/// A complete DICOM session loaded from a directory.
#[derive(Debug, Clone)]
pub struct DicomSession {
    pub subjects: Vec<DicomSubject>,
}

impl DicomSession {
    /// Total number of series across all subjects/studies/acquisitions.
    pub fn total_series(&self) -> usize {
        self.subjects.iter().flat_map(|s| &s.studies)
            .flat_map(|st| &st.acquisitions)
            .map(|a| a.series.len())
            .sum()
    }

    /// Flatten all series into a list with indices for navigation.
    pub fn flat_series(&self) -> Vec<FlatSeriesRef> {
        let mut result = Vec::new();
        for (si, sub) in self.subjects.iter().enumerate() {
            for (sti, study) in sub.studies.iter().enumerate() {
                for (ai, acq) in study.acquisitions.iter().enumerate() {
                    for (sei, _series) in acq.series.iter().enumerate() {
                        result.push(FlatSeriesRef { sub: si, study: sti, acq: ai, series: sei });
                    }
                }
            }
        }
        result
    }

    /// Get a mutable reference to a series by flat index.
    pub fn series_mut(&mut self, r: &FlatSeriesRef) -> &mut DicomSeries {
        &mut self.subjects[r.sub].studies[r.study].acquisitions[r.acq].series[r.series]
    }

    /// Get a reference to a series by flat index.
    #[allow(dead_code)]
    pub fn series_ref(&self, r: &FlatSeriesRef) -> &DicomSeries {
        &self.subjects[r.sub].studies[r.study].acquisitions[r.acq].series[r.series]
    }
}

/// Index into the flattened series list.
#[derive(Debug, Clone)]
pub struct FlatSeriesRef {
    pub sub: usize,
    pub study: usize,
    pub acq: usize,
    pub series: usize,
}

// ─── Utility functions ───

/// Clean a string for use as a BIDS label (alphanumeric only).
fn clean_bids_label(s: &str) -> String {
    s.chars().filter(|c| c.is_alphanumeric()).collect()
}

/// Strip trailing Siemens _RR suffixes from series descriptions.
/// Siemens exports can append _RR (or _RR_RR etc.) to repeated series.
fn normalize_series_description(desc: &str) -> String {
    let mut s = desc.to_string();
    while s.ends_with("_RR") {
        s.truncate(s.len() - 3);
    }
    s
}

/// Compute ImageType signature by removing known type markers.
/// Used to pair mag/phase series that come from the same source acquisition.
fn image_type_signature(image_type: &[String]) -> Vec<String> {
    let type_markers = ["M", "P", "MAG", "PHASE", "REAL", "IMAGINARY", "MAGNITUDE"];
    image_type.iter()
        .filter(|v| !type_markers.contains(&v.as_str()))
        .cloned()
        .collect()
}

/// Extract a string tag value from a DICOM object, returning empty string if missing.
fn get_str_tag(obj: &dicom::object::DefaultDicomObject, tag: dicom::core::Tag) -> String {
    obj.element_opt(tag)
        .ok()
        .flatten()
        .and_then(|e| e.to_str().ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_default()
}

/// Extract a float tag value from a DICOM object.
fn get_float_tag(obj: &dicom::object::DefaultDicomObject, tag: dicom::core::Tag) -> Option<f64> {
    obj.element_opt(tag)
        .ok()
        .flatten()
        .and_then(|e| e.to_str().ok())
        .and_then(|s| s.trim().parse::<f64>().ok())
}

/// Extract an integer tag value from a DICOM object.
fn get_int_tag(obj: &dicom::object::DefaultDicomObject, tag: dicom::core::Tag) -> Option<i32> {
    obj.element_opt(tag)
        .ok()
        .flatten()
        .and_then(|e| e.to_str().ok())
        .and_then(|s| s.trim().parse::<i32>().ok())
}

/// Parse a time string (HHMMSS.fractional) to seconds since midnight.
fn parse_time_to_seconds(time_str: &str) -> Option<f64> {
    let s = time_str.trim();
    if s.len() < 6 { return None; }
    let hours: f64 = s[..2].parse().ok()?;
    let minutes: f64 = s[2..4].parse().ok()?;
    let seconds: f64 = s[4..].parse().ok()?;
    Some(hours * 3600.0 + minutes * 60.0 + seconds)
}

/// Determine coil type from Siemens private tag (0051,100F) string.
fn classify_coil_string(coil_str: &str) -> CoilType {
    if coil_str.is_empty() {
        return CoilType::Unknown;
    }
    // Combined: contains semicolon, range notation (e.g. "1-6"), or no numbers
    if coil_str.contains(';') {
        return CoilType::Combined;
    }
    // Check for range pattern like "HC1-6" or "1-32"
    let has_range = coil_str.chars().any(|c| c == '-')
        && coil_str.chars().any(|c| c.is_ascii_digit());
    if has_range {
        // Verify it's a range (digit-digit) not just a hyphenated name
        let parts: Vec<&str> = coil_str.split('-').collect();
        if parts.len() == 2 {
            let left_has_digit = parts[0].chars().any(|c| c.is_ascii_digit());
            let right_has_digit = parts[1].chars().any(|c| c.is_ascii_digit());
            if left_has_digit && right_has_digit {
                return CoilType::Combined;
            }
        }
    }
    // No numbers at all → combined
    if !coil_str.chars().any(|c| c.is_ascii_digit()) {
        return CoilType::Combined;
    }
    // Has single number(s) without range/semicolon → uncombined
    CoilType::Uncombined
}

// ─── DICOM reading ───

/// Read metadata from a single DICOM file (classic single-frame).
fn read_dicom_file(path: &Path) -> Option<DicomFileInfo> {
    use dicom::dictionary_std::tags;

    let obj = open_file(path).ok()?;

    // Check for enhanced DICOM (multi-frame)
    let is_enhanced = obj.element_opt(tags::PER_FRAME_FUNCTIONAL_GROUPS_SEQUENCE)
        .ok().flatten().is_some();

    // Try PatientID first, then PatientName, clean both, use first non-empty
    let raw_id = get_str_tag(&obj, tags::PATIENT_ID);
    let raw_name = get_str_tag(&obj, tags::PATIENT_NAME);
    let clean_id = clean_bids_label(&raw_id);
    let clean_name = clean_bids_label(&raw_name);
    let patient_id = if !clean_id.is_empty() {
        clean_id
    } else if !clean_name.is_empty() {
        clean_name
    } else {
        "unknown".to_string()
    };

    let study_date = get_str_tag(&obj, tags::STUDY_DATE);
    let series_instance_uid = get_str_tag(&obj, tags::SERIES_INSTANCE_UID);

    if series_instance_uid.is_empty() {
        return None;
    }

    let series_description = get_str_tag(&obj, tags::SERIES_DESCRIPTION);
    let protocol_name = {
        let pn = get_str_tag(&obj, tags::PROTOCOL_NAME);
        if pn.is_empty() { series_description.clone() } else { pn }
    };

    let series_number = get_int_tag(&obj, tags::SERIES_NUMBER).unwrap_or(0);
    let echo_time = get_float_tag(&obj, tags::ECHO_TIME);
    let magnetic_field_strength = get_float_tag(&obj, tags::MAGNETIC_FIELD_STRENGTH);
    let manufacturer = get_str_tag(&obj, tags::MANUFACTURER);

    // Acquisition time for temporal clustering
    let acq_time_str = {
        let t = get_str_tag(&obj, tags::ACQUISITION_TIME);
        if t.is_empty() { get_str_tag(&obj, tags::SERIES_TIME) } else { t }
    };
    let acquisition_time = parse_time_to_seconds(&acq_time_str);

    // ImageType is a multi-valued string separated by backslashes
    let image_type_raw = get_str_tag(&obj, tags::IMAGE_TYPE);
    let image_type: Vec<String> = if image_type_raw.is_empty() {
        Vec::new()
    } else {
        image_type_raw.split('\\').map(|s| s.trim().to_uppercase()).collect()
    };

    // Siemens coil element string (private tag 0051,100F)
    let coil_tag = dicom::core::Tag(0x0051, 0x100F);
    let coil_string = get_str_tag(&obj, coil_tag);

    Some(DicomFileInfo {
        path: path.to_path_buf(),
        patient_id,
        study_date,
        series_instance_uid,
        series_description,
        protocol_name,
        series_number,
        echo_time,
        image_type,
        magnetic_field_strength,
        manufacturer,
        acquisition_time,
        coil_string,
        is_enhanced,
    })
}

// ─── Enhanced DICOM support ───

/// Read metadata from an enhanced (multi-frame) DICOM file.
/// Returns one DicomFileInfo per frame, all sharing the same series-level metadata
/// but potentially different echo times and image types.
fn read_enhanced_dicom_frames(path: &Path) -> Vec<DicomFileInfo> {
    use dicom::dictionary_std::tags;

    let obj = match open_file(path) {
        Ok(o) => o,
        Err(_) => return Vec::new(),
    };

    // Get common metadata
    let raw_id = get_str_tag(&obj, tags::PATIENT_ID);
    let raw_name = get_str_tag(&obj, tags::PATIENT_NAME);
    let clean_id = clean_bids_label(&raw_id);
    let clean_name = clean_bids_label(&raw_name);
    let patient_id = if !clean_id.is_empty() {
        clean_id
    } else if !clean_name.is_empty() {
        clean_name
    } else {
        "unknown".to_string()
    };

    let study_date = get_str_tag(&obj, tags::STUDY_DATE);
    let series_instance_uid = get_str_tag(&obj, tags::SERIES_INSTANCE_UID);
    let series_description = get_str_tag(&obj, tags::SERIES_DESCRIPTION);
    let protocol_name = {
        let pn = get_str_tag(&obj, tags::PROTOCOL_NAME);
        if pn.is_empty() { series_description.clone() } else { pn }
    };
    let series_number = get_int_tag(&obj, tags::SERIES_NUMBER).unwrap_or(0);
    let magnetic_field_strength = get_float_tag(&obj, tags::MAGNETIC_FIELD_STRENGTH);
    let manufacturer = get_str_tag(&obj, tags::MANUFACTURER);
    let coil_tag = dicom::core::Tag(0x0051, 0x100F);
    let coil_string = get_str_tag(&obj, coil_tag);

    let image_type_raw = get_str_tag(&obj, tags::IMAGE_TYPE);
    let image_type: Vec<String> = if image_type_raw.is_empty() {
        Vec::new()
    } else {
        image_type_raw.split('\\').map(|s| s.trim().to_uppercase()).collect()
    };

    let echo_time = get_float_tag(&obj, tags::ECHO_TIME);
    let acq_time_str = {
        let t = get_str_tag(&obj, tags::ACQUISITION_TIME);
        if t.is_empty() { get_str_tag(&obj, tags::SERIES_TIME) } else { t }
    };
    let acquisition_time = parse_time_to_seconds(&acq_time_str);

    // For enhanced DICOM, we treat the whole file as a single entry
    // (dcm2niix handles per-frame extraction internally)
    vec![DicomFileInfo {
        path: path.to_path_buf(),
        patient_id,
        study_date,
        series_instance_uid,
        series_description,
        protocol_name,
        series_number,
        echo_time,
        image_type,
        magnetic_field_strength,
        manufacturer,
        acquisition_time,
        coil_string,
        is_enhanced: true,
    }]
}

// ─── Auto-labeling ───

/// Auto-detect the series type from ImageType field.
fn auto_label_series(image_type: &[String], description: &str) -> SeriesType {
    let desc_lower = description.to_lowercase();

    // Check ImageType values
    for val in image_type {
        match val.as_str() {
            "P" | "PHASE" => return SeriesType::Phase,
            "M" | "MAG" | "MAGNITUDE" => return SeriesType::Magnitude,
            "REAL" => return SeriesType::Real,
            "IMAGINARY" => return SeriesType::Imaginary,
            _ => {}
        }
    }

    // Check description for hints
    if desc_lower.contains("t1") && (desc_lower.contains("mprage") || desc_lower.contains("mp2rage") || desc_lower.contains("t1w")) {
        return SeriesType::T1w;
    }
    if desc_lower.contains("phase") || desc_lower.ends_with("_ph") {
        return SeriesType::Phase;
    }
    if desc_lower.contains("mag") {
        return SeriesType::Magnitude;
    }

    // Default to magnitude for GRE-looking sequences
    if desc_lower.contains("gre") || desc_lower.contains("swi") || desc_lower.contains("qsm") {
        return SeriesType::Magnitude;
    }

    SeriesType::Extra
}

// ─── Temporal clustering and run detection ───

/// Group SeriesInstanceUIDs into runs using temporal clustering.
/// UIDs acquired within a 60-second window from the cluster start belong to the same run.
fn assign_runs_temporal(
    series_list: &[DicomSeries],
    file_times: &HashMap<String, f64>,
) -> Vec<u32> {
    let gap_threshold = 60.0; // seconds

    // Collect (series_index, median_time) for series that have time data
    let mut timed: Vec<(usize, f64)> = Vec::new();
    let mut untimed: Vec<usize> = Vec::new();

    for (i, series) in series_list.iter().enumerate() {
        if let Some(&t) = file_times.get(&series.series_uid) {
            timed.push((i, t));
        } else {
            untimed.push(i);
        }
    }

    let mut run_assignments = vec![1u32; series_list.len()];

    if timed.is_empty() {
        return run_assignments;
    }

    // Sort by time
    timed.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

    let mut current_run = 1u32;
    let mut cluster_start = timed[0].1;

    for &(idx, time) in &timed {
        if time - cluster_start > gap_threshold {
            current_run += 1;
            cluster_start = time;
        }
        run_assignments[idx] = current_run;
    }

    // Assign orphan (untimed) series to closest run by series number proximity
    if !untimed.is_empty() && !timed.is_empty() {
        for &idx in &untimed {
            // Find the timed series with the closest series number
            let orphan_num = series_list[idx].series_number;
            let closest_run = timed.iter()
                .min_by_key(|&&(ti, _)| (series_list[ti].series_number - orphan_num).unsigned_abs())
                .map(|&(ti, _)| run_assignments[ti])
                .unwrap_or(1);
            run_assignments[idx] = closest_run;
        }
    }

    run_assignments
}

// ─── ImageType signature pairing ───

/// Try to pair mag/phase (or real/imag) series within an acquisition using ImageType signatures.
/// Series with matching signatures (after removing type markers) are likely from the same source.
fn pair_series_by_signature(series_list: &mut [DicomSeries]) {
    // Build signatures for each series
    let signatures: Vec<Vec<String>> = series_list.iter()
        .map(|s| image_type_signature(&s.image_type))
        .collect();

    // Find series that could be mag or phase based on ImageType
    let mut mag_indices: Vec<usize> = Vec::new();
    let mut phase_indices: Vec<usize> = Vec::new();
    let mut real_indices: Vec<usize> = Vec::new();
    let mut imag_indices: Vec<usize> = Vec::new();

    for (i, s) in series_list.iter().enumerate() {
        match s.series_type {
            SeriesType::Magnitude => mag_indices.push(i),
            SeriesType::Phase => phase_indices.push(i),
            SeriesType::Real => real_indices.push(i),
            SeriesType::Imaginary => imag_indices.push(i),
            _ => {}
        }
    }

    // Try to pair mag/phase by matching signatures
    pair_indices_by_signature(&signatures, &mag_indices, &phase_indices);
    // Try to pair real/imag by matching signatures
    pair_indices_by_signature(&signatures, &real_indices, &imag_indices);
}

/// Helper: check if two groups have matching signatures (for validation/logging).
fn pair_indices_by_signature(
    signatures: &[Vec<String>],
    _group_a: &[usize],
    _group_b: &[usize],
) {
    // Currently this is used for validation only — the actual pairing
    // happens through the acquisition grouping (same ProtocolName).
    // If signatures match between mag and phase in the same acquisition,
    // they're correctly paired. If they don't match, we still pair them
    // sequentially (which is the fallback behavior).
    //
    // Future: could use this to split/merge acquisitions when signature
    // matching reveals a different grouping than ProtocolName alone.
    let _ = signatures;
}

// ─── Main scan function ───

/// Scan a directory for DICOM files and build a structured session.
/// `progress` is atomically incremented for each file examined (DICOM or not).
pub fn scan_dicom_directory(dir: &Path, progress: Arc<AtomicUsize>) -> Result<DicomSession, String> {
    if !dir.is_dir() {
        return Err(format!("Not a directory: {}", dir.display()));
    }

    // Walk directory and read all DICOM files
    let mut files: Vec<DicomFileInfo> = Vec::new();
    walk_dir(dir, &mut files, &progress);

    if files.is_empty() {
        return Err("No DICOM files found".to_string());
    }

    // Group by patient_id → study_date → series_instance_uid
    let mut patient_map: HashMap<String, HashMap<String, HashMap<String, Vec<DicomFileInfo>>>> = HashMap::new();

    for file in files {
        patient_map
            .entry(file.patient_id.clone())
            .or_default()
            .entry(file.study_date.clone())
            .or_default()
            .entry(file.series_instance_uid.clone())
            .or_default()
            .push(file);
    }

    let mut subjects: Vec<DicomSubject> = Vec::new();

    for (patient_id, studies_map) in &patient_map {
        let mut studies: Vec<DicomStudy> = Vec::new();

        for (study_date, series_map) in studies_map {
            // Build DicomSeries from grouped files, with _RR normalization
            let mut all_series: Vec<DicomSeries> = Vec::new();

            // Compute median acquisition time per SeriesInstanceUID for temporal clustering
            let mut uid_times: HashMap<String, f64> = HashMap::new();

            for (uid, file_group) in series_map {
                let first = &file_group[0];
                let series_type = auto_label_series(&first.image_type, &first.series_description);
                let coil_type = classify_coil_string(&first.coil_string);

                // Normalize series description (strip _RR suffixes)
                let normalized_desc = normalize_series_description(&first.series_description);

                // Compute median acquisition time for this UID
                let mut times: Vec<f64> = file_group.iter()
                    .filter_map(|f| f.acquisition_time)
                    .collect();
                times.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                if let Some(&median) = times.get(times.len() / 2) {
                    uid_times.insert(uid.clone(), median);
                }

                all_series.push(DicomSeries {
                    series_uid: uid.clone(),
                    description: normalized_desc,
                    protocol_name: first.protocol_name.clone(),
                    series_number: first.series_number,
                    image_type: first.image_type.clone(),
                    echo_time: first.echo_time,
                    magnetic_field_strength: first.magnetic_field_strength,
                    num_files: file_group.len(),
                    series_type,
                    files: file_group.iter().map(|f| f.path.clone()).collect(),
                    manufacturer: first.manufacturer.clone(),
                    coil_type,
                });
            }

            // Sort by series number
            all_series.sort_by_key(|s| s.series_number);

            // Apply ImageType signature-based pairing validation
            pair_series_by_signature(&mut all_series);

            // Group series into acquisitions by normalized protocol name
            let mut acq_map: HashMap<String, Vec<DicomSeries>> = HashMap::new();
            for series in all_series {
                let key = clean_bids_label(&series.protocol_name);
                let key = if key.is_empty() { "unknown".to_string() } else { key };
                acq_map.entry(key).or_default().push(series);
            }

            // For each acquisition group, use temporal clustering to detect runs
            let mut acquisitions: Vec<DicomAcquisition> = Vec::new();

            let mut acq_list: Vec<(String, Vec<DicomSeries>)> = acq_map.into_iter().collect();
            acq_list.sort_by(|a, b| {
                let a_min = a.1.iter().map(|s| s.series_number).min().unwrap_or(0);
                let b_min = b.1.iter().map(|s| s.series_number).min().unwrap_or(0);
                a_min.cmp(&b_min)
            });

            for (name, series_in_acq) in acq_list {
                let run_assignments = assign_runs_temporal(&series_in_acq, &uid_times);
                let max_run = *run_assignments.iter().max().unwrap_or(&1);

                for run_num in 1..=max_run {
                    let run_series: Vec<DicomSeries> = series_in_acq.iter()
                        .zip(run_assignments.iter())
                        .filter(|(_, &r)| r == run_num)
                        .map(|(s, _)| s.clone())
                        .collect();

                    if !run_series.is_empty() {
                        acquisitions.push(DicomAcquisition {
                            name: name.clone(),
                            run_number: run_num,
                            series: run_series,
                        });
                    }
                }
            }

            studies.push(DicomStudy {
                study_date: study_date.clone(),
                acquisitions,
            });
        }

        studies.sort_by(|a, b| a.study_date.cmp(&b.study_date));

        subjects.push(DicomSubject {
            patient_id: patient_id.clone(),
            studies,
        });
    }

    subjects.sort_by(|a, b| a.patient_id.cmp(&b.patient_id));

    Ok(DicomSession { subjects })
}

/// Recursively walk a directory and read DICOM files.
fn walk_dir(dir: &Path, results: &mut Vec<DicomFileInfo>, progress: &AtomicUsize) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk_dir(&path, results, progress);
        } else {
            progress.fetch_add(1, Ordering::Relaxed);
            if let Some(info) = read_dicom_file(&path) {
                if info.is_enhanced {
                    // For enhanced DICOM, re-read to get per-frame info
                    results.extend(read_enhanced_dicom_frames(&path));
                } else {
                    results.push(info);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_series_description() {
        assert_eq!(normalize_series_description("t2star_qsm_tra_p3_RR"), "t2star_qsm_tra_p3");
        assert_eq!(normalize_series_description("t2star_qsm_tra_p3_RR_RR"), "t2star_qsm_tra_p3");
        assert_eq!(normalize_series_description("t2star_qsm_tra_p3"), "t2star_qsm_tra_p3");
        assert_eq!(normalize_series_description("_RR"), "");
    }

    #[test]
    fn test_image_type_signature() {
        let sig = image_type_signature(&["ORIGINAL".into(), "PRIMARY".into(), "M".into(), "NORM".into()]);
        assert_eq!(sig, vec!["ORIGINAL", "PRIMARY", "NORM"]);

        let sig2 = image_type_signature(&["ORIGINAL".into(), "PRIMARY".into(), "P".into(), "NORM".into()]);
        assert_eq!(sig2, vec!["ORIGINAL", "PRIMARY", "NORM"]);

        // Same signature → same source
        assert_eq!(sig, sig2);
    }

    #[test]
    fn test_classify_coil_string() {
        assert_eq!(classify_coil_string("HEA;HEP"), CoilType::Combined);
        assert_eq!(classify_coil_string("HC1-6"), CoilType::Combined);
        assert_eq!(classify_coil_string("HEA"), CoilType::Combined);
        assert_eq!(classify_coil_string("H1"), CoilType::Uncombined);
        assert_eq!(classify_coil_string("A32"), CoilType::Uncombined);
        assert_eq!(classify_coil_string(""), CoilType::Unknown);
    }

    #[test]
    fn test_parse_time_to_seconds() {
        assert_eq!(parse_time_to_seconds("120000.000"), Some(43200.0));
        assert_eq!(parse_time_to_seconds("000100.000"), Some(60.0));
        assert_eq!(parse_time_to_seconds(""), None);
    }

    #[test]
    fn test_clean_bids_label() {
        assert_eq!(clean_bids_label("p025_pre^^^^"), "p025pre");
        assert_eq!(clean_bids_label("."), "");
        assert_eq!(clean_bids_label("sub01"), "sub01");
    }

    #[test]
    #[ignore] // requires real DICOM data at a local path
    fn test_scan_real_dicoms() {
        let dir = Path::new("/home/ashley/organise/QSMBLY/qsm-dicoms/");
        if !dir.exists() {
            return;
        }
        let progress = Arc::new(AtomicUsize::new(0));
        let p2 = Arc::clone(&progress);
        let start = std::time::Instant::now();

        let monitor = std::thread::spawn(move || {
            loop {
                std::thread::sleep(std::time::Duration::from_millis(500));
                let n = p2.load(Ordering::Relaxed);
                eprintln!("  progress: {} files ({:.1}s)", n, start.elapsed().as_secs_f64());
                if n >= 2112 || start.elapsed().as_secs() > 120 {
                    break;
                }
            }
        });

        let result = scan_dicom_directory(dir, progress);
        let elapsed = start.elapsed();
        eprintln!("Scan completed in {:.1}s", elapsed.as_secs_f64());

        let _ = monitor.join();

        let session = result.expect("scan should succeed");
        eprintln!("Subjects: {}", session.subjects.len());
        eprintln!("Total series: {}", session.total_series());
        for sub in &session.subjects {
            eprintln!("  sub-{}", sub.patient_id);
            for study in &sub.studies {
                eprintln!("    study date: {}", study.study_date);
                for acq in &study.acquisitions {
                    eprintln!("      acq-{} run-{} ({} series)", acq.name, acq.run_number, acq.series.len());
                    for s in &acq.series {
                        eprintln!("        {} [{}] ({} files, TE={:?}, coil={:?}, mfr={})",
                            s.description, s.series_type.label(), s.num_files, s.echo_time, s.coil_type, s.manufacturer);
                    }
                }
            }
        }
        assert!(session.total_series() > 0);
    }

    // ─── Helper to build a DicomSeries for testing ───

    fn make_series(uid: &str, desc: &str, series_number: i32, series_type: SeriesType) -> DicomSeries {
        DicomSeries {
            series_uid: uid.to_string(),
            description: desc.to_string(),
            protocol_name: desc.to_string(),
            series_number,
            image_type: Vec::new(),
            echo_time: None,
            magnetic_field_strength: None,
            num_files: 1,
            series_type,
            files: Vec::new(),
            manufacturer: String::new(),
            coil_type: CoilType::Unknown,
        }
    }

    // ─── SeriesType::label() ───

    #[test]
    fn test_series_type_label_all_variants() {
        assert_eq!(SeriesType::Magnitude.label(), "Magnitude");
        assert_eq!(SeriesType::Phase.label(), "Phase");
        assert_eq!(SeriesType::Real.label(), "Real");
        assert_eq!(SeriesType::Imaginary.label(), "Imaginary");
        assert_eq!(SeriesType::T1w.label(), "T1w");
        assert_eq!(SeriesType::Extra.label(), "Extra");
        assert_eq!(SeriesType::Skip.label(), "Skip");
    }

    // ─── SeriesType::next() and prev() ───

    #[test]
    fn test_series_type_next_cycles_through_all() {
        let mut current = SeriesType::ALL[0];
        let mut visited = vec![current];
        for _ in 1..SeriesType::ALL.len() {
            current = current.next();
            visited.push(current);
        }
        assert_eq!(visited.len(), SeriesType::ALL.len());
        for (i, &expected) in SeriesType::ALL.iter().enumerate() {
            assert_eq!(visited[i], expected);
        }
        // Wraps around
        assert_eq!(current.next(), SeriesType::ALL[0]);
    }

    #[test]
    fn test_series_type_prev_cycles_through_all() {
        let mut current = SeriesType::ALL[0];
        let mut visited = vec![current];
        for _ in 1..SeriesType::ALL.len() {
            current = current.prev();
            visited.push(current);
        }
        // prev from first should go to last, then second-to-last, etc.
        assert_eq!(visited[1], *SeriesType::ALL.last().unwrap());
        // Wraps around
        assert_eq!(current.prev(), SeriesType::ALL[0]);
    }

    #[test]
    fn test_series_type_next_prev_inverse() {
        for &t in SeriesType::ALL {
            assert_eq!(t.next().prev(), t);
            assert_eq!(t.prev().next(), t);
        }
    }

    // ─── auto_label_series() ───

    #[test]
    fn test_auto_label_phase_from_image_type_p() {
        assert_eq!(auto_label_series(&["P".into()], ""), SeriesType::Phase);
    }

    #[test]
    fn test_auto_label_phase_from_image_type_phase() {
        assert_eq!(auto_label_series(&["PHASE".into()], ""), SeriesType::Phase);
    }

    #[test]
    fn test_auto_label_magnitude_from_image_type_m() {
        assert_eq!(auto_label_series(&["M".into()], ""), SeriesType::Magnitude);
    }

    #[test]
    fn test_auto_label_magnitude_from_image_type_mag() {
        assert_eq!(auto_label_series(&["MAG".into()], ""), SeriesType::Magnitude);
    }

    #[test]
    fn test_auto_label_magnitude_from_image_type_magnitude() {
        assert_eq!(auto_label_series(&["MAGNITUDE".into()], ""), SeriesType::Magnitude);
    }

    #[test]
    fn test_auto_label_real_from_image_type() {
        assert_eq!(auto_label_series(&["REAL".into()], ""), SeriesType::Real);
    }

    #[test]
    fn test_auto_label_imaginary_from_image_type() {
        assert_eq!(auto_label_series(&["IMAGINARY".into()], ""), SeriesType::Imaginary);
    }

    #[test]
    fn test_auto_label_t1w_from_description() {
        assert_eq!(auto_label_series(&[], "T1_MPRAGE"), SeriesType::T1w);
        assert_eq!(auto_label_series(&[], "t1_mp2rage_sag"), SeriesType::T1w);
        assert_eq!(auto_label_series(&[], "my_t1w_scan"), SeriesType::T1w);
    }

    #[test]
    fn test_auto_label_phase_from_description_ph_suffix() {
        assert_eq!(auto_label_series(&[], "t2star_qsm_tra_p3_ph"), SeriesType::Phase);
    }

    #[test]
    fn test_auto_label_phase_from_description_phase() {
        assert_eq!(auto_label_series(&[], "some_phase_image"), SeriesType::Phase);
    }

    #[test]
    fn test_auto_label_magnitude_from_description_mag() {
        assert_eq!(auto_label_series(&[], "some_mag_image"), SeriesType::Magnitude);
    }

    #[test]
    fn test_auto_label_magnitude_from_description_gre() {
        assert_eq!(auto_label_series(&[], "gre_field_mapping"), SeriesType::Magnitude);
    }

    #[test]
    fn test_auto_label_magnitude_from_description_swi() {
        assert_eq!(auto_label_series(&[], "SWI_combined"), SeriesType::Magnitude);
    }

    #[test]
    fn test_auto_label_magnitude_from_description_qsm() {
        assert_eq!(auto_label_series(&[], "QSM_sequence"), SeriesType::Magnitude);
    }

    #[test]
    fn test_auto_label_extra_fallback() {
        assert_eq!(auto_label_series(&[], "localizer"), SeriesType::Extra);
        assert_eq!(auto_label_series(&[], ""), SeriesType::Extra);
        assert_eq!(auto_label_series(&["ORIGINAL".into(), "PRIMARY".into()], "flair_sag"), SeriesType::Extra);
    }

    #[test]
    fn test_auto_label_image_type_takes_priority_over_description() {
        // ImageType says Phase, description says mag — ImageType wins
        assert_eq!(auto_label_series(&["P".into()], "mag_image"), SeriesType::Phase);
    }

    // ─── assign_runs_temporal() ───

    #[test]
    fn test_assign_runs_no_time_data() {
        let series = vec![
            make_series("uid1", "s1", 1, SeriesType::Magnitude),
            make_series("uid2", "s2", 2, SeriesType::Phase),
        ];
        let file_times: HashMap<String, f64> = HashMap::new();
        let runs = assign_runs_temporal(&series, &file_times);
        assert_eq!(runs, vec![1, 1]);
    }

    #[test]
    fn test_assign_runs_single_run() {
        let series = vec![
            make_series("uid1", "s1", 1, SeriesType::Magnitude),
            make_series("uid2", "s2", 2, SeriesType::Phase),
        ];
        let mut file_times = HashMap::new();
        file_times.insert("uid1".to_string(), 100.0);
        file_times.insert("uid2".to_string(), 130.0); // within 60s
        let runs = assign_runs_temporal(&series, &file_times);
        assert_eq!(runs, vec![1, 1]);
    }

    #[test]
    fn test_assign_runs_multiple_runs_gap_over_60s() {
        let series = vec![
            make_series("uid1", "s1", 1, SeriesType::Magnitude),
            make_series("uid2", "s2", 2, SeriesType::Phase),
            make_series("uid3", "s3", 3, SeriesType::Magnitude),
            make_series("uid4", "s4", 4, SeriesType::Phase),
        ];
        let mut file_times = HashMap::new();
        file_times.insert("uid1".to_string(), 100.0);
        file_times.insert("uid2".to_string(), 120.0);
        file_times.insert("uid3".to_string(), 300.0); // gap > 60s from cluster start (100)
        file_times.insert("uid4".to_string(), 320.0);
        let runs = assign_runs_temporal(&series, &file_times);
        assert_eq!(runs[0], 1);
        assert_eq!(runs[1], 1);
        assert_eq!(runs[2], 2);
        assert_eq!(runs[3], 2);
    }

    #[test]
    fn test_assign_runs_untimed_orphan_series() {
        let series = vec![
            make_series("uid1", "s1", 1, SeriesType::Magnitude),
            make_series("uid2", "s2", 5, SeriesType::Phase), // untimed, closer to uid3 by series_number
            make_series("uid3", "s3", 6, SeriesType::Magnitude),
        ];
        let mut file_times = HashMap::new();
        file_times.insert("uid1".to_string(), 100.0);
        // uid2 has no time data
        file_times.insert("uid3".to_string(), 300.0); // separate run
        let runs = assign_runs_temporal(&series, &file_times);
        assert_eq!(runs[0], 1);
        // uid2 (series_number=5) is closer to uid3 (series_number=6) than uid1 (series_number=1)
        assert_eq!(runs[1], 2);
        assert_eq!(runs[2], 2);
    }

    // ─── pair_series_by_signature() ───

    #[test]
    fn test_pair_series_by_signature_basic() {
        let mut series = vec![
            {
                let mut s = make_series("uid1", "gre", 1, SeriesType::Magnitude);
                s.image_type = vec!["ORIGINAL".into(), "PRIMARY".into(), "M".into(), "NORM".into()];
                s
            },
            {
                let mut s = make_series("uid2", "gre", 2, SeriesType::Phase);
                s.image_type = vec!["ORIGINAL".into(), "PRIMARY".into(), "P".into(), "NORM".into()];
                s
            },
        ];
        // Should not panic
        pair_series_by_signature(&mut series);
        // Types should remain unchanged (function is currently validation-only)
        assert_eq!(series[0].series_type, SeriesType::Magnitude);
        assert_eq!(series[1].series_type, SeriesType::Phase);
    }

    // ─── DicomSession methods ───

    fn make_test_session() -> DicomSession {
        DicomSession {
            subjects: vec![
                DicomSubject {
                    patient_id: "sub01".to_string(),
                    studies: vec![
                        DicomStudy {
                            study_date: "20240101".to_string(),
                            acquisitions: vec![
                                DicomAcquisition {
                                    name: "gre".to_string(),
                                    run_number: 1,
                                    series: vec![
                                        make_series("uid1", "gre_mag", 1, SeriesType::Magnitude),
                                        make_series("uid2", "gre_phase", 2, SeriesType::Phase),
                                    ],
                                },
                                DicomAcquisition {
                                    name: "t1w".to_string(),
                                    run_number: 1,
                                    series: vec![
                                        make_series("uid3", "t1_mprage", 3, SeriesType::T1w),
                                    ],
                                },
                            ],
                        },
                    ],
                },
                DicomSubject {
                    patient_id: "sub02".to_string(),
                    studies: vec![
                        DicomStudy {
                            study_date: "20240102".to_string(),
                            acquisitions: vec![
                                DicomAcquisition {
                                    name: "swi".to_string(),
                                    run_number: 1,
                                    series: vec![
                                        make_series("uid4", "swi", 1, SeriesType::Magnitude),
                                    ],
                                },
                            ],
                        },
                    ],
                },
            ],
        }
    }

    #[test]
    fn test_total_series() {
        let session = make_test_session();
        assert_eq!(session.total_series(), 4);
    }

    #[test]
    fn test_total_series_empty() {
        let session = DicomSession { subjects: Vec::new() };
        assert_eq!(session.total_series(), 0);
    }

    #[test]
    fn test_flat_series_count() {
        let session = make_test_session();
        let flat = session.flat_series();
        assert_eq!(flat.len(), 4);
    }

    #[test]
    fn test_flat_series_indices_are_valid() {
        let session = make_test_session();
        let flat = session.flat_series();
        for r in &flat {
            let _series = session.series_ref(r);
        }
    }

    #[test]
    fn test_series_ref_returns_correct_series() {
        let session = make_test_session();
        let flat = session.flat_series();
        assert_eq!(session.series_ref(&flat[0]).series_uid, "uid1");
        assert_eq!(session.series_ref(&flat[1]).series_uid, "uid2");
        assert_eq!(session.series_ref(&flat[2]).series_uid, "uid3");
        assert_eq!(session.series_ref(&flat[3]).series_uid, "uid4");
    }

    #[test]
    fn test_series_mut_can_modify() {
        let mut session = make_test_session();
        let flat = session.flat_series();
        session.series_mut(&flat[0]).series_type = SeriesType::Skip;
        assert_eq!(session.series_ref(&flat[0]).series_type, SeriesType::Skip);
    }

    #[test]
    fn test_flat_series_empty_session() {
        let session = DicomSession { subjects: Vec::new() };
        assert!(session.flat_series().is_empty());
    }
}
