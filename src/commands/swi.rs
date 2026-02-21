use qsm_core::nifti_io;

use crate::cli::SwiArgs;
use crate::error::QsmxtError;

pub fn execute(args: SwiArgs) -> crate::Result<()> {
    let phase_nifti = nifti_io::read_nifti_file(&args.phase)
        .map_err(|e| QsmxtError::NiftiIo(e))?;
    let mag_nifti = nifti_io::read_nifti_file(&args.magnitude)
        .map_err(|e| QsmxtError::NiftiIo(e))?;
    let mask_nifti = nifti_io::read_nifti_file(&args.mask)
        .map_err(|e| QsmxtError::NiftiIo(e))?;

    let (nx, ny, nz) = phase_nifti.dims;
    let (vsx, vsy, vsz) = phase_nifti.voxel_size;
    let mask: Vec<u8> = mask_nifti.data.iter().map(|&v| if v > 0.5 { 1u8 } else { 0u8 }).collect();

    println!("Computing SWI ({}x{}x{})", nx, ny, nz);

    let swi = qsm_core::swi::calculate_swi_default(
        &phase_nifti.data, &mag_nifti.data, &mask,
        nx, ny, nz, vsx, vsy, vsz,
    );

    nifti_io::save_nifti_to_file(
        &args.output,
        &swi,
        phase_nifti.dims,
        phase_nifti.voxel_size,
        &phase_nifti.affine,
    )
    .map_err(|e| QsmxtError::NiftiIo(e))?;

    println!("SWI saved to {}", args.output.display());

    if args.mip {
        let mip = qsm_core::swi::create_mip_default(&swi, nx, ny, nz);
        let mip_path = args
            .mip_output
            .unwrap_or_else(|| args.output.with_extension("mip.nii"));
        nifti_io::save_nifti_to_file(
            &mip_path,
            &mip,
            phase_nifti.dims,
            phase_nifti.voxel_size,
            &phase_nifti.affine,
        )
        .map_err(|e| QsmxtError::NiftiIo(e))?;
        println!("MIP saved to {}", mip_path.display());
    }

    Ok(())
}
