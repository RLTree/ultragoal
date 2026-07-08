use std::path::Path;

pub(in crate::cli::typed_boundaries) fn reject_receipt_collision(
    receipt: &Path,
) -> Result<(), String> {
    for reserved in [
        super::super::FOUNDATIONAL_SURFACE_INVENTORY_REL,
        super::super::PACKAGE_SURFACE_INVENTORY_REL,
    ] {
        if same_root_relative_path(receipt, Path::new(reserved)) {
            return Err(format!(
                "{}: typed boundaries receipt must not overwrite reserved inventory artifact {reserved}",
                receipt.display()
            ));
        }
    }
    Ok(())
}

fn same_root_relative_path(left: &Path, right: &Path) -> bool {
    left.components().eq(right.components())
}
