//! Profiles ("codeplugs") for Unication G-series pagers and generation of the
//! on-pager radio database.

pub mod clone;
pub mod compare;
pub mod conv_knob;
pub mod convert141;
pub mod digital_knob;
pub mod edit;
pub mod installer;
pub mod knob_copy;
pub mod knob_remove;
pub mod knob_system;
pub mod row;
pub mod rr_import;
pub mod scan_knob;
pub mod settings;
pub mod source;
pub mod summary;
pub mod system_edit;
pub mod zone_edit;

pub use source::{Profile, Templates};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("sqlite: {0}")]
    Sqlite(#[from] sqlite2::Error),
    #[error("i/o: {0}")]
    Io(#[from] std::io::Error),
    #[error("zip: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("invalid profile: {0}")]
    Invalid(String),
    #[error("Unication templates not found in {0} (point at the PPS DataBase folder)")]
    Templates(std::path::PathBuf),
    #[error("PPS installer: {0}")]
    Installer(String),
    #[error("not supported yet (refusing rather than guessing): {0}")]
    Unsupported(String),
    #[error("conversion failed:\n  {}", .0.join("\n  "))]
    Conversion(Vec<String>),
}
