use super::common::{load_nifti, load_mask, save_nifti, save_mask};
use crate::cli::{BfAlgorithmArg, BgremoveArgs};

pub fn execute(args: BgremoveArgs) -> crate::Result<()> {
    let field_nifti = load_nifti(&args.input)?;
    let (mask, _) = load_mask(&args.mask)?;

    let (nx, ny, nz) = field_nifti.dims;
    let (vsx, vsy, vsz) = field_nifti.voxel_size;

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

    save_nifti(&args.output, &local_field, &field_nifti)?;
    println!("Local field saved to {}", args.output.display());

    if let Some(ref mask_out) = args.output_mask {
        save_mask(mask_out, &eroded_mask, &field_nifti)?;
        println!("Eroded mask saved to {}", mask_out.display());
    }

    Ok(())
}
