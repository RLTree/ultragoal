use super::contracts::RedCatalogError;
use std::ffi::OsString;
use std::fs::File;
use std::path::Path;

#[cfg(unix)]
#[path = "filesystem_descriptor.rs"]
mod descriptor;
#[cfg(not(unix))]
mod descriptor {
    use super::{File, OsString, Path, RedCatalogError, error};

    pub(super) struct BoundRoot;
    impl BoundRoot {
        pub(super) fn open(_: &Path) -> Result<Self, RedCatalogError> {
            Err(error("red_catalog_platform_unsupported", "repository"))
        }
        pub(super) fn file(&self) -> &File {
            unreachable!("unsupported platform has no bound root")
        }
        pub(super) fn validate(&self) -> Result<(), RedCatalogError> {
            Err(error("red_catalog_platform_unsupported", "repository"))
        }
    }
    pub(super) fn open_directory(
        _: &File,
        _: &[u8],
        detail: &str,
    ) -> Result<File, RedCatalogError> {
        Err(error("red_catalog_platform_unsupported", detail))
    }
    pub(super) fn read_regular(
        _: &File,
        _: &[u8],
        detail: &str,
    ) -> Result<Vec<u8>, RedCatalogError> {
        Err(error("red_catalog_platform_unsupported", detail))
    }
    pub(super) fn directory_names(_: &File) -> Result<Vec<OsString>, RedCatalogError> {
        Err(error("red_catalog_platform_unsupported", "fixtures/red"))
    }
    pub(super) fn directory_snapshot(_: &File) -> Result<(), RedCatalogError> {
        Err(error("red_catalog_platform_unsupported", "fixtures/red"))
    }
}

const MAX_RED_PACKET_COUNT: usize = 8_192;

pub(super) struct RedPacketSource {
    pub(super) relative: String,
    pub(super) bytes: Vec<u8>,
}

pub(super) struct RedCatalogFilesystem {
    root: descriptor::BoundRoot,
}

impl RedCatalogFilesystem {
    pub(super) fn open(root: &Path) -> Result<Self, RedCatalogError> {
        Ok(Self {
            root: descriptor::BoundRoot::open(root)?,
        })
    }

    pub(super) fn packet_sources(&self) -> Result<Vec<RedPacketSource>, RedCatalogError> {
        let fixtures = descriptor::open_directory(self.root.file(), b"fixtures", "fixtures")?;
        let directory = descriptor::open_directory(&fixtures, b"red", "fixtures/red")?;
        let before = descriptor::directory_snapshot(&directory)?;
        let names = descriptor::directory_names(&directory)?;
        if names.len() > MAX_RED_PACKET_COUNT {
            return Err(error("red_catalog_fixture_count_exceeded", "fixtures/red"));
        }
        let packets = names
            .into_iter()
            .map(|name| packet_source(&directory, name))
            .collect::<Result<Vec<_>, _>>()?;
        if before != descriptor::directory_snapshot(&directory)? {
            return Err(error(
                "red_catalog_fixture_directory_changed",
                "fixtures/red",
            ));
        }
        Ok(packets)
    }

    pub(super) fn catalog_bytes(&self) -> Result<Vec<u8>, RedCatalogError> {
        let templates = descriptor::open_directory(self.root.file(), b"templates", "templates")?;
        descriptor::read_regular(
            &templates,
            b"RED_FIXTURES.json",
            "templates/RED_FIXTURES.json",
        )
    }

    pub(super) fn validate(&self) -> Result<(), RedCatalogError> {
        self.root.validate()
    }
}

fn packet_source(directory: &File, name: OsString) -> Result<RedPacketSource, RedCatalogError> {
    let value = name
        .to_str()
        .filter(|value| value.ends_with(".json"))
        .ok_or_else(|| {
            error(
                "red_catalog_fixture_path_unknown",
                &format!("fixtures/red/{}", name.to_string_lossy()),
            )
        })?;
    let relative = format!("fixtures/red/{value}");
    let bytes = descriptor::read_regular(directory, name.as_encoded_bytes(), &relative)?;
    Ok(RedPacketSource { relative, bytes })
}

fn error(code: &'static str, path: &str) -> RedCatalogError {
    RedCatalogError::new(code, Some(path.to_string()))
}
