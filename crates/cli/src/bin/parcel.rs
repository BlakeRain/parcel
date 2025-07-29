use clap::Parser;

use parcel_cli::args::Args;

fn main() {
    let args = Args::parse();
    println!("{args:#?}");
}
