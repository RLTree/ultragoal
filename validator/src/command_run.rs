use crate::Args;

pub fn run(args: Args) -> Result<i32, String> {
    crate::cli::successor_public::run_public(&args.root, args.outcome)
}
