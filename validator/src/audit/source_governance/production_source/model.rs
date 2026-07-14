#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ProductionSource {
    pub(crate) relative: String,
    pub(crate) bytes: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ProductionSourceSet {
    pub(crate) sources: Vec<ProductionSource>,
    pub(crate) failures: Vec<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct CompilePossibility {
    pub(super) production: bool,
    pub(super) test: bool,
}

impl CompilePossibility {
    pub(super) const BOTH: Self = Self {
        production: true,
        test: true,
    };

    pub(super) fn and(self, other: Self) -> Self {
        Self {
            production: self.production && other.production,
            test: self.test && other.test,
        }
    }
}
