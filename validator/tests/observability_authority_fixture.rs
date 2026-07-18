use std::fs;
use std::path::Path;

pub(crate) fn copy_current_inventory_inputs(live: &Path, root: &Path) {
    crate::authority_inputs::copy_declared_files(live, root);
    copy_file(live, root, "LANE_REGISTRY.json");
    copy_file(live, root, "templates/LANE_REGISTRY.json");
}

fn copy_file(live: &Path, root: &Path, relative: &str) {
    let target = root.join(relative);
    fs::create_dir_all(target.parent().expect("fixture authority parent"))
        .expect("create fixture authority parent");
    fs::copy(live.join(relative), target).expect("copy fixture authority input");
}
