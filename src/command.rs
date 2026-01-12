mod add;
mod completions;
mod config;
mod list;
mod man;
mod remove;
mod trust;
mod untrust;

pub(crate) use add::run as add;
pub(crate) use completions::run as completions;
pub(crate) use config::run as config;
pub(crate) use list::run as list;
pub(crate) use man::run as man;
pub(crate) use remove::run as remove;
pub(crate) use trust::run as trust;
pub(crate) use untrust::run as untrust;
