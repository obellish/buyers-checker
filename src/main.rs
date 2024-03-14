use std::{
	path::PathBuf,
	sync::atomic::{AtomicUsize, Ordering::SeqCst},
};

use clap::Parser;
use csv_async::AsyncReaderBuilder;
use futures::{future::BoxFuture, FutureExt as _, TryStreamExt as _};
use miette::{IntoDiagnostic as _, Result};
use tokio::{
	fs::{read_dir, File},
	runtime::Builder,
};
use tokio_stream::wrappers::ReadDirStream;
use tracing::{event, Level};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

static THREAD_ID: AtomicUsize = AtomicUsize::new(1);

const BARCODE_INDEX: usize = 4;

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
	let log_filter_layer = EnvFilter::try_from_default_env()
		.or_else(|_| EnvFilter::try_new("debug"))
		.into_diagnostic()?;

	let log_fmt_layer = fmt::layer()
		.pretty()
		.with_ansi(true)
		.with_thread_ids(true)
		.with_thread_names(true)
		.with_file(false);

	let output_file = tokio::fs::File::create("./output.log")
		.await
		.into_diagnostic()?
		.into_std()
		.await;

	let log_fs_layer = fmt::layer()
		.compact()
		.with_ansi(false)
		.with_writer(output_file);

	tracing_subscriber::registry()
		.with(log_filter_layer)
		.with(log_fmt_layer)
		.with(log_fs_layer)
		.try_init()
		.into_diagnostic()?;

	let args = Args::try_parse().into_diagnostic()?;

	if let Some(file) = args.file_path {
		check_file(file).await?;
	} else if let Some(folder) = args.folder_path {
		check_directory(folder).await?;
	} else {
		panic!("No file or folder path was given.");
	}

	Ok(())
}

fn check_directory(path: PathBuf) -> BoxFuture<'static, Result<()>> {
	event!(Level::INFO, ?path, "checking directory");
	assert!(path.is_dir());
	async move {
		let mut dir_stream = ReadDirStream::new(read_dir(path).await.into_diagnostic()?);

		let mut futures = Vec::new();

		while let Some(entry) = dir_stream.try_next().await.into_diagnostic()? {
			let path = entry.path();

			if path.is_file() {
				futures.push(tokio::spawn(async move { check_file(path).await }));
			} else {
				event!(Level::WARN, ?path, "path is not a file");
			}
		}

		futures::future::try_join_all(futures)
			.await
			.into_diagnostic()?
			.into_iter()
			.collect::<Result<()>>()?;

		Ok(())
	}
	.boxed()
}

fn check_file(path: PathBuf) -> BoxFuture<'static, Result<()>> {
	assert!(path.is_file());
	event!(Level::DEBUG, ?path, "checking file");
	async move {
		let file = File::open(path).await.into_diagnostic()?;

		let mut reader = AsyncReaderBuilder::new()
			.has_headers(false)
			.create_reader(file);

		let mut record_stream = reader.records();

		let mut records = Vec::new();

		while let Some(item) = record_stream.try_next().await.into_diagnostic()? {
			records.extend(
				item.get(BARCODE_INDEX)
					.map(|s| {
						let mut out = s.to_owned();
						out.pop();
						out
					})
					.filter(|s| !s.is_empty()),
			);
		}

		let cloned_records = records.clone();

		records.sort();

		assert_eq!(records.len(), cloned_records.len());

		for i in 0..records.len() - 1 {
			let original = cloned_records.get(i);
			let sorted = records.get(i);
			event!(Level::TRACE, ?original, ?sorted, %i, "matching records");
			assert_eq!(original, sorted);
		}

		Ok(())
	}
	.boxed()
}

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
struct Args {
	/// The file path to check for.
	///
	/// Must be valid CSV.
	#[arg(short, long, value_name = "FILE")]
	file_path: Option<PathBuf>,
	/// The folder path to read from.
	///
	/// Must have valid CSV files.
	#[arg(long, value_name = "DIRECTORY")]
	folder_path: Option<PathBuf>,
}
