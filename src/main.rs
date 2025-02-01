use clap::Parser;

use jsonpp::Args;

pub fn main() {
    env_logger::init();
    let args = Args::parse();
    args.execute();
}
