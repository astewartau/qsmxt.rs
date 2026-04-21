use super::common::{load_mask, save_mask};
use crate::cli::CloseArgs;

pub fn execute(args: CloseArgs) -> crate::Result<()> {
    let (mask, nifti) = load_mask(&args.input)?;
    let (nx, ny, nz) = nifti.dims;

    println!("Morphological close ({}x{}x{}, radius={})", nx, ny, nz, args.radius);
    let result = qsm_core::utils::morphological_close(&mask, nx, ny, nz, args.radius as i32);

    save_mask(&args.output, &result, &nifti)?;
    println!("Closed mask saved to {}", args.output.display());
    Ok(())
}
