mod file_checks;
mod tracing_setup;
mod util;

use std::path::PathBuf;

use clap::Parser;

pub use self::{
	file_checks::{check_directory, check_file},
	tracing_setup::setup_tracing,
};

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
pub struct Args {
	/// The file path to check for
	///
	/// Must be valid CSV
	#[arg(short, long, value_name = "FILE")]
	pub file_path: Option<PathBuf>,
	/// The folder path to read from
	///
	/// Must have valid CSV files
	#[arg(long, value_name = "DIRECTORY")]
	pub folder_path: Option<PathBuf>,
	/// The folder to output files to
	#[arg(short, long, value_name = "DIRECTORY")]
	pub output_folder: PathBuf,
}
