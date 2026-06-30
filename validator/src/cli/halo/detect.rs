use std::path::Path;

pub(super) struct AppObservation {
    pub(super) observed: bool,
    pub(super) path_class: &'static str,
    pub(super) identifier: String,
    pub(super) version: String,
}

pub(super) fn app(path: &Path) -> AppObservation {
    let info = path.join("Contents/Info.plist");
    let text = std::fs::read_to_string(&info).unwrap_or_default();
    AppObservation {
        observed: path.is_dir() && info.is_file(),
        path_class: if path.starts_with("/Applications") {
            "system_applications"
        } else {
            "configured_local_path"
        },
        identifier: plist_value(&text, "CFBundleIdentifier").unwrap_or_else(|| "unknown".into()),
        version: plist_value(&text, "CFBundleVersion").unwrap_or_else(|| "unknown".into()),
    }
}

pub(super) fn command_on_path(name: &str) -> bool {
    command_on_paths(name, std::env::var_os("PATH"))
}

pub(super) fn command_on_paths(name: &str, paths: Option<std::ffi::OsString>) -> bool {
    let Some(paths) = paths else {
        return false;
    };
    std::env::split_paths(&paths).any(|dir| dir.join(name).is_file())
}

fn plist_value(text: &str, key: &str) -> Option<String> {
    let marker = format!("<key>{key}</key>");
    let after = text.split(&marker).nth(1)?;
    let value = after.split("<string>").nth(1)?.split("</string>").next()?;
    Some(value.to_string())
}
