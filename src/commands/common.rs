//! Shared helpers for standalone CLI commands.

use std::path::Path;
use qsm_core::nifti_io::{self, NiftiData};
use crate::error::QsmxtError;

/// Load a NIfTI file with error mapping.
pub fn load_nifti(path: &Path) -> crate::Result<NiftiData> {
    nifti_io::read_nifti_file(path)
        .map_err(|e| QsmxtError::NiftiIo(format!("{}: {}", path.display(), e)))
}

/// Load a binary mask from a NIfTI file (threshold at 0.5).
pub fn load_mask(path: &Path) -> crate::Result<(Vec<u8>, NiftiData)> {
    let nifti = load_nifti(path)?;
    let mask: Vec<u8> = nifti.data.iter().map(|&v| if v > 0.5 { 1u8 } else { 0u8 }).collect();
    Ok((mask, nifti))
}

/// Save a f64 volume to NIfTI, preserving geometry from a reference.
pub fn save_nifti(path: &Path, data: &[f64], reference: &NiftiData) -> crate::Result<()> {
    nifti_io::save_nifti_to_file(path, data, reference.dims, reference.voxel_size, &reference.affine)
        .map_err(|e| QsmxtError::NiftiIo(e))
}

/// Save a u8 mask as f64 NIfTI, preserving geometry from a reference.
pub fn save_mask(path: &Path, mask: &[u8], reference: &NiftiData) -> crate::Result<()> {
    let data: Vec<f64> = mask.iter().map(|&m| m as f64).collect();
    save_nifti(path, &data, reference)
}
