use super::common::{load_nifti, save_mask};
use crate::cli::{MaskArgs, ThresholdMethod};
use crate::error::QsmxtError;
use crate::pipeline::phase;

pub fn execute(args: MaskArgs) -> crate::Result<()> {
    let nifti = load_nifti(&args.input)?;
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

    let mut mask: Vec<u8> = nifti.data.iter()
        .map(|&v| if v > threshold { 1u8 } else { 0u8 })
        .collect();

    if args.erosions > 0 {
        mask = phase::erode_mask(&mask, nx, ny, nz, args.erosions);
    }

    save_mask(&args.output, &mask, &nifti)?;

    let count: usize = mask.iter().map(|&m| m as usize).sum();
    println!(
        "Mask saved to {} ({} voxels, {:.1}%)",
        args.output.display(), count, 100.0 * count as f64 / mask.len() as f64
    );
    Ok(())
}
