use super::common::run_mask_operation;
use crate::cli::CloseArgs;

pub fn execute(args: CloseArgs) -> crate::Result<()> {
    let radius = args.radius;
    run_mask_operation(&args.input, &args.output, "Morphological close", |mask, nx, ny, nz| {
        qsm_core::utils::morphological_close(mask, nx, ny, nz, radius as i32)
    })
}
