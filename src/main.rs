use clap::Parser;

use jsonpp::Args;

pub fn main() {
    let args = Args::parse();
    args.execute();
}
