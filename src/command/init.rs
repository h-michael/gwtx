use crate::cli::InitArgs;
use crate::error::Result;

pub(crate) fn run(args: InitArgs) -> Result<()> {
    crate::init::run(args)
}
