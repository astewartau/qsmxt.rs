use super::common::run_mask_operation;
use crate::cli::FillHolesArgs;

pub fn execute(args: FillHolesArgs) -> crate::Result<()> {
    let max_size = args.max_size;
    run_mask_operation(&args.input, &args.output, "Filling holes", |mask, nx, ny, nz| {
        qsm_core::utils::fill_holes(mask, nx, ny, nz, max_size)
    })
}
