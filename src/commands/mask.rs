use qsm_core::nifti_io;

use crate::cli::{MaskArgs, ThresholdMethod};
use crate::error::QsmxtError;
use crate::pipeline::phase;

pub fn execute(args: MaskArgs) -> crate::Result<()> {
    let nifti = nifti_io::read_nifti_file(&args.input)
        .map_err(|e| QsmxtError::NiftiIo(e))?;

    let (nx, ny, nz) = nifti.dims;

    let threshold = match args.method {
        ThresholdMethod::Otsu => {
            let t = qsm_core::utils::otsu_threshold(&nifti.data, 256);
            println!("Otsu threshold: {:.4}", t);
            t
        }
        ThresholdMethod::Value => {
            args.threshold.ok_or_else(|| {
                QsmxtError::Config("--threshold required when method=value".to_string())
            })?
        }
    };

    let mut mask: Vec<u8> = nifti
        .data
        .iter()
        .map(|&v| if v > threshold { 1u8 } else { 0u8 })
        .collect();

    if args.erosions > 0 {
        mask = phase::erode_mask(&mask, nx, ny, nz, args.erosions);
    }

    let mask_f64: Vec<f64> = mask.iter().map(|&m| m as f64).collect();
    nifti_io::save_nifti_to_file(
        &args.output,
        &mask_f64,
        nifti.dims,
        nifti.voxel_size,
        &nifti.affine,
    )
    .map_err(|e| QsmxtError::NiftiIo(e))?;

    let count: usize = mask.iter().map(|&m| m as usize).sum();
    println!(
        "Mask saved to {} ({} voxels, {:.1}%)",
        args.output.display(),
        count,
        100.0 * count as f64 / mask.len() as f64
    );

    Ok(())
}
