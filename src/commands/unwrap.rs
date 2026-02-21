use qsm_core::nifti_io;

use crate::cli::{UnwrapAlgorithmArg, UnwrapArgs};
use crate::error::QsmxtError;
use crate::pipeline::phase;

pub fn execute(args: UnwrapArgs) -> crate::Result<()> {
    let phase_nifti = nifti_io::read_nifti_file(&args.input)
        .map_err(|e| QsmxtError::NiftiIo(e))?;
    let mask_nifti = nifti_io::read_nifti_file(&args.mask)
        .map_err(|e| QsmxtError::NiftiIo(e))?;

    let (nx, ny, nz) = phase_nifti.dims;
    let (vsx, vsy, vsz) = phase_nifti.voxel_size;
    let mask: Vec<u8> = mask_nifti.data.iter().map(|&v| if v > 0.5 { 1u8 } else { 0u8 }).collect();

    println!("Unwrapping phase ({:?}, {}x{}x{})", args.algorithm, nx, ny, nz);

    let unwrapped = match args.algorithm {
        UnwrapAlgorithmArg::Laplacian => {
            qsm_core::unwrap::laplacian_unwrap(&phase_nifti.data, &mask, nx, ny, nz, vsx, vsy, vsz)
        }
        UnwrapAlgorithmArg::Romeo => {
            let mag = if let Some(ref mag_path) = args.magnitude {
                let m = nifti_io::read_nifti_file(mag_path)
                    .map_err(|e| QsmxtError::NiftiIo(e))?;
                m.data
            } else {
                vec![1.0f64; phase_nifti.data.len()]
            };

            let weights = qsm_core::unwrap::calculate_weights_single_echo(
                &phase_nifti.data, &mag, &mask, nx, ny, nz,
            );

            let mut phase_mut = phase_nifti.data.clone();
            let mut mask_mut = mask.clone();
            let (si, sj, sk) = phase::mask_center_of_mass(&mask, nx, ny, nz);

            qsm_core::region_grow::grow_region_unwrap(
                &mut phase_mut, &weights, &mut mask_mut,
                nx, ny, nz, si, sj, sk,
            );

            phase_mut
        }
    };

    nifti_io::save_nifti_to_file(
        &args.output,
        &unwrapped,
        phase_nifti.dims,
        phase_nifti.voxel_size,
        &phase_nifti.affine,
    )
    .map_err(|e| QsmxtError::NiftiIo(e))?;

    println!("Unwrapped phase saved to {}", args.output.display());
    Ok(())
}
