use std::path::PathBuf;
pub struct Options {
    pub root: PathBuf,
    pub registry: String,
    pub inventory: bool,
    pub help: bool,
}
#[derive(Debug)]
pub struct OptionsError(String);
impl std::fmt::Display for OptionsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
pub fn read() -> Result<Options, OptionsError> {
    let mut options = Options {
        root: std::env::current_dir().map_err(|e| OptionsError(e.to_string()))?,
        registry: "docs/legibility/registry.json".into(),
        inventory: false,
        help: false,
    };
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--root" => {
                options.root = args
                    .next()
                    .ok_or_else(|| OptionsError("--root needs a path".into()))?
                    .into()
            }
            "--registry" => {
                options.registry = args
                    .next()
                    .ok_or_else(|| OptionsError("--registry needs a relative path".into()))?
            }
            "--inventory" => options.inventory = true,
            "--help" => options.help = true,
            _ => return Err(OptionsError(format!("unknown argument: {arg}"))),
        }
    }
    options.root = options
        .root
        .canonicalize()
        .map_err(|e| OptionsError(e.to_string()))?;
    Ok(options)
}
