pub mod build;
pub mod types;
pub mod flash;
pub mod resources;

use std::path::PathBuf;

use build::build_bundle;
use clap::{Parser, Subcommand};
use flash::flash_bundle;

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
    config: PathBuf,
    #[arg(short, long)]
    lasercan_rev1_bootloader_check: bool
  },
  /// Flash a bundle to an MCU target
  Flash {
    bundle: PathBuf,
    #[arg(short, long)]
    chip: String,
  }
}

fn main() {
  let cli = Cli::parse();

  match &cli.command {
    Commands::Build { output, firmware, bootloader, config, lasercan_rev1_bootloader_check } => {
      build_bundle(output, firmware, bootloader, config, *lasercan_rev1_bootloader_check).unwrap();
    },
    Commands::Flash { bundle, chip } => {
      flash_bundle(bundle, chip).unwrap();
    },
  }
}
