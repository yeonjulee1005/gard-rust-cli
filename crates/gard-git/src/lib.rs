pub mod ci;
pub mod diff;
pub mod hook;

pub use ci::CiProvider;
pub use diff::detect_new_packages;
pub use hook::{install as install_hook, uninstall as uninstall_hook};
