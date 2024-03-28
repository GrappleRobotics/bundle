pub mod build;
pub mod types;

use std::path::{Path, PathBuf};

use build::build_bundle;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
  #[command(subcommand)]
  command: Commands
}

#[derive(Subcommand)]
enum Commands {
  /// Build a bundle
  Build {
    output: PathBuf,
    #[arg(short, long)]
    firmware: PathBuf,
    #[arg(short, long)]
    bootloader: PathBuf,
    #[arg(short, long)]
    svd: PathBuf,
    #[arg(short, long)]
    config: PathBuf,
  }
}

fn main() {
  let cli = Cli::parse();

  match &cli.command {
    Commands::Build { output, firmware, bootloader, svd, config } => {
      build_bundle(output, firmware, bootloader, svd, config).unwrap();
    },
  }
}
