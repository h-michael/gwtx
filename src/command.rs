mod add;
mod completions;
mod man;
mod validate;

pub(crate) use add::run as add;
pub(crate) use completions::run as completions;
pub(crate) use man::run as man;
pub(crate) use validate::run as validate;
