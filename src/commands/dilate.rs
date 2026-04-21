use super::common::{load_mask, save_mask};
use crate::cli::DilateArgs;
use crate::pipeline::phase;

pub fn execute(args: DilateArgs) -> crate::Result<()> {
    let (mask, nifti) = load_mask(&args.input)?;
    let (nx, ny, nz) = nifti.dims;

    println!("Dilating mask ({}x{}x{}, {} iterations)", nx, ny, nz, args.iterations);
    let result = phase::dilate_mask(&mask, nx, ny, nz, args.iterations);

    save_mask(&args.output, &result, &nifti)?;
    println!("Dilated mask saved to {}", args.output.display());
    Ok(())
}
