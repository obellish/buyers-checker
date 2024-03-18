use std::sync::atomic::{AtomicUsize, Ordering::SeqCst};

use buyers_checker::{check_directory, check_file, setup_tracing, Args};
use clap::Parser;
use miette::{IntoDiagnostic as _, Result};
use tokio::runtime::Builder;

static THREAD_ID: AtomicUsize = AtomicUsize::new(1);

fn main() -> Result<()> {
	Builder::new_multi_thread()
		.thread_name_fn(|| {
			let id = THREAD_ID.fetch_add(1, SeqCst) + 1;
			let output = String::from("buyers-checker-pool-");
			output + &id.to_string()
		})
		.on_thread_stop(|| {
			THREAD_ID.fetch_sub(1, SeqCst);
		})
		.build()
		.into_diagnostic()?
		.block_on(run())
}

async fn run() -> Result<()> {
	let args = Args::try_parse().into_diagnostic()?;

	setup_tracing(&args.output_folder).await?;

	if let Some(file) = args.file_path {
		check_file(&file, &args.output_folder).await?;
	} else if let Some(folder) = args.folder_path {
		check_directory(folder, args.output_folder).await?;
	} else {
		panic!("No file or folder path was given.");
	}

	Ok(())
}
