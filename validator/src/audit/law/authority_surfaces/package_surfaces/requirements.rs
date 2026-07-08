use super::RequiredSurface;

pub(super) const REQUIRED_SURFACES: &[RequiredSurface] = &[
    RequiredSurface {
        role: "source",
        rel: "validator/src/audit/law/authority_surfaces/package_surfaces/cargo.rs",
        package_inventory_required: true,
    },
    RequiredSurface {
        role: "source",
        rel: "validator/src/audit/law/authority_surfaces/package_surfaces/mod.rs",
        package_inventory_required: true,
    },
    RequiredSurface {
        role: "source",
        rel: "validator/src/audit/law/authority_surfaces/package_surfaces/requirements.rs",
        package_inventory_required: true,
    },
    RequiredSurface {
        role: "source",
        rel: "validator/src/audit/law/authority_surfaces/package_surfaces/row.rs",
        package_inventory_required: true,
    },
    RequiredSurface {
        role: "source",
        rel: "validator/src/audit/law/authority_surfaces/package_surfaces/source/mod.rs",
        package_inventory_required: true,
    },
];
