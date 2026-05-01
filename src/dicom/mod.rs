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

/// Clean a string for use as a BIDS label (alphanumeric only).
fn clean_bids_label(s: &str) -> String {
    s.chars().filter(|c| c.is_alphanumeric()).collect()
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

/// Read metadata from a single DICOM file.
fn read_dicom_file(path: &Path) -> Option<DicomFileInfo> {
    use dicom::dictionary_std::tags;

    let obj = open_file(path).ok()?;

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

    // ImageType is a multi-valued string separated by backslashes
    let image_type_raw = get_str_tag(&obj, tags::IMAGE_TYPE);
    let image_type: Vec<String> = if image_type_raw.is_empty() {
        Vec::new()
    } else {
        image_type_raw.split('\\').map(|s| s.trim().to_uppercase()).collect()
    };

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
    })
}

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
            // Build DicomSeries from grouped files
            let mut all_series: Vec<DicomSeries> = Vec::new();
            for (uid, file_group) in series_map {
                let first = &file_group[0];
                let series_type = auto_label_series(&first.image_type, &first.series_description);
                all_series.push(DicomSeries {
                    series_uid: uid.clone(),
                    description: first.series_description.clone(),
                    protocol_name: first.protocol_name.clone(),
                    series_number: first.series_number,
                    image_type: first.image_type.clone(),
                    echo_time: first.echo_time,
                    magnetic_field_strength: first.magnetic_field_strength,
                    num_files: file_group.len(),
                    series_type,
                    files: file_group.iter().map(|f| f.path.clone()).collect(),
                });
            }

            // Sort by series number
            all_series.sort_by_key(|s| s.series_number);

            // Group series into acquisitions by protocol name
            let mut acq_map: HashMap<String, Vec<DicomSeries>> = HashMap::new();
            for series in all_series {
                let key = clean_bids_label(&series.protocol_name);
                let key = if key.is_empty() { "unknown".to_string() } else { key };
                acq_map.entry(key).or_default().push(series);
            }

            // Convert to acquisitions with run numbers
            let mut acquisitions: Vec<DicomAcquisition> = Vec::new();
            let mut run_counts: HashMap<String, u32> = HashMap::new();

            let mut acq_list: Vec<(String, Vec<DicomSeries>)> = acq_map.into_iter().collect();
            acq_list.sort_by(|a, b| {
                let a_min = a.1.iter().map(|s| s.series_number).min().unwrap_or(0);
                let b_min = b.1.iter().map(|s| s.series_number).min().unwrap_or(0);
                a_min.cmp(&b_min)
            });

            for (name, series) in acq_list {
                let count = run_counts.entry(name.clone()).or_insert(0);
                *count += 1;
                acquisitions.push(DicomAcquisition {
                    name: name.clone(),
                    run_number: *count,
                    series,
                });
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

#[cfg(test)]
mod tests {
    use super::*;

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

        // Monitor progress in background
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
                        eprintln!("        {} [{}] ({} files, TE={:?})", s.description, s.series_type.label(), s.num_files, s.echo_time);
                    }
                }
            }
        }
        assert!(session.total_series() > 0);
    }
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
                results.push(info);
            }
        }
    }
}
