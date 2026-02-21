use qsm_core::nifti_io;

use crate::cli::{BfAlgorithmArg, BgremoveArgs};
use crate::error::QsmxtError;

pub fn execute(args: BgremoveArgs) -> crate::Result<()> {
    let field_nifti = nifti_io::read_nifti_file(&args.input)
        .map_err(|e| QsmxtError::NiftiIo(e))?;
    let mask_nifti = nifti_io::read_nifti_file(&args.mask)
        .map_err(|e| QsmxtError::NiftiIo(e))?;

    let (nx, ny, nz) = field_nifti.dims;
    let (vsx, vsy, vsz) = field_nifti.voxel_size;
    let mask: Vec<u8> = mask_nifti.data.iter().map(|&v| if v > 0.5 { 1u8 } else { 0u8 }).collect();

    println!("Background removal ({:?}, {}x{}x{})", args.algorithm, nx, ny, nz);

    let (local_field, eroded_mask) = match args.algorithm {
        BfAlgorithmArg::Vsharp => {
            qsm_core::bgremove::vsharp_default(&field_nifti.data, &mask, nx, ny, nz, vsx, vsy, vsz)
        }
        BfAlgorithmArg::Pdf => {
            let lf = qsm_core::bgremove::pdf_default(&field_nifti.data, &mask, nx, ny, nz, vsx, vsy, vsz);
            (lf, mask.clone())
        }
        BfAlgorithmArg::Lbv => {
            qsm_core::bgremove::lbv_default(&field_nifti.data, &mask, nx, ny, nz, vsx, vsy, vsz)
        }
        BfAlgorithmArg::Ismv => {
            qsm_core::bgremove::ismv_default(&field_nifti.data, &mask, nx, ny, nz, vsx, vsy, vsz)
        }
    };

    nifti_io::save_nifti_to_file(
        &args.output,
        &local_field,
        field_nifti.dims,
        field_nifti.voxel_size,
        &field_nifti.affine,
    )
    .map_err(|e| QsmxtError::NiftiIo(e))?;

    println!("Local field saved to {}", args.output.display());

    if let Some(ref mask_out) = args.output_mask {
        let mask_f64: Vec<f64> = eroded_mask.iter().map(|&m| m as f64).collect();
        nifti_io::save_nifti_to_file(
            mask_out,
            &mask_f64,
            field_nifti.dims,
            field_nifti.voxel_size,
            &field_nifti.affine,
        )
        .map_err(|e| QsmxtError::NiftiIo(e))?;
        println!("Eroded mask saved to {}", mask_out.display());
    }

    Ok(())
}
