mod add;
mod completions;
mod config;
mod man;
mod remove;

pub(crate) use add::run as add;
pub(crate) use completions::run as completions;
pub(crate) use config::run as config;
pub(crate) use man::run as man;
pub(crate) use remove::run as remove;
