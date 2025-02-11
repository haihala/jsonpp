use clap::Parser;

use json_preprocessor::Args;

pub fn main() {
    env_logger::init();
    let args = Args::parse();
    args.execute();
}
