use super::*;

pub(crate) fn expose_current_ultragoal() {
    static PATH: OnceLock<std::ffi::OsString> = OnceLock::new();
    let path = PATH.get_or_init(|| {
        let directory =
            std::env::temp_dir().join(format!("hul-routine-typed-runner-{}", std::process::id()));
        fs::create_dir_all(&directory).unwrap();
        let link = directory.join("ultragoal");
        let executable = std::env::current_exe().unwrap();
        if fs::read_link(&link).ok().as_ref() != Some(&executable) {
            let _ = fs::remove_file(&link);
            symlink(executable, &link).unwrap();
        }
        std::env::join_paths(std::iter::once(directory).chain(std::env::split_paths(
            &std::env::var_os("PATH").unwrap_or_default(),
        )))
        .unwrap()
    });
    unsafe { std::env::set_var("PATH", path) };
}
