use super::common::{load_mask, save_mask};
use crate::cli::FillHolesArgs;

pub fn execute(args: FillHolesArgs) -> crate::Result<()> {
    let (mask, nifti) = load_mask(&args.input)?;
    let (nx, ny, nz) = nifti.dims;

    println!("Filling holes ({}x{}x{}, max_size={})", nx, ny, nz, args.max_size);
    let result = qsm_core::utils::fill_holes(&mask, nx, ny, nz, args.max_size);

    let filled: u32 = result.iter().zip(mask.iter()).map(|(&r, &m)| if r > m { 1 } else { 0 }).sum();
    save_mask(&args.output, &result, &nifti)?;
    println!("Filled mask saved to {} ({} voxels filled)", args.output.display(), filled);
    Ok(())
}
