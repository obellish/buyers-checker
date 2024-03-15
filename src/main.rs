use std::{
	path::{Path, PathBuf},
	sync::atomic::{AtomicUsize, Ordering::SeqCst},
};

use clap::Parser;
use csv_async::{AsyncReaderBuilder, AsyncWriterBuilder, StringRecord};
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

	let args = Args::try_parse().into_diagnostic()?;

	// We don't care if the output directory didn't exist.
	_ = tokio::fs::remove_dir_all(&args.output_folder).await;

	tokio::fs::create_dir_all(&args.output_folder)
		.await
		.into_diagnostic()?;

	let output_file = tokio::fs::File::create("./outputs/log_output.log")
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

	if let Some(file) = args.file_path {
		check_file(&file, &args.output_folder).await?;
	} else if let Some(folder) = args.folder_path {
		check_directory(folder, args.output_folder).await?;
	} else {
		panic!("No file or folder path was given.");
	}

	Ok(())
}

fn check_directory<'a>(path: PathBuf, output_path: PathBuf) -> BoxFuture<'a, Result<()>> {
	event!(Level::INFO, ?path, "checking directory");
	assert!(path.is_dir());
	async move {
		let mut dir_stream = ReadDirStream::new(read_dir(path).await.into_diagnostic()?);

		let mut futures = Vec::new();

		while let Some(entry) = dir_stream.try_next().await.into_diagnostic()? {
			let path = entry.path();

			if path.is_file() {
				let output_path = output_path.clone();
				futures.push(tokio::spawn(async move {
					check_file(&path, &output_path).await
				}));
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

fn check_file<'a>(input_path: &'a Path, output_path: &'a Path) -> BoxFuture<'a, Result<()>> {
	assert!(input_path.is_file());
	event!(Level::DEBUG, ?input_path, "checking file");
	async move {
		let file = File::open(&input_path).await.into_diagnostic()?;

		let mut reader = AsyncReaderBuilder::new()
			.has_headers(false)
			.create_reader(file);

		let records = reader
			.records()
			.try_filter_map(|item| {
				let index = item.get(0).and_then(|s| s.parse::<usize>().ok());
				let barcode = item.get(BARCODE_INDEX).and_then(|s| {
					let mut out = s.to_owned();
					out.pop();
					out.parse::<u64>().ok()
				});

				futures::future::ok(index.zip(barcode))
			})
			.try_collect::<Vec<_>>()
			.await
			.into_diagnostic()?;

		// let output_path = format!(
		// 	"{output_paht}/{}",
		// 	input_path
		// 		.file_name()
		// 		.and_then(|s| s.to_str())
		// 		.unwrap_or("FAIL.txt")
		// );

		let output_path = output_path.join(
			input_path
				.file_name()
				.and_then(|s| s.to_str())
				.unwrap_or("FAIL.txt"),
		);

		let output_file_future = File::create(output_path);

		let mut output_writer = AsyncWriterBuilder::new()
			.flexible(false)
			.has_headers(false)
			.create_writer(output_file_future.await.into_diagnostic()?);

		for (i, (record_index, record)) in records.iter().enumerate().skip(1) {
			let Some((_, before)) = records.get(i - 1) else {
				continue;
			};

			if record - 1 != *before {
				event!(Level::ERROR, ?input_path, %record_index, %record);

				let string_record = [record_index.to_string(), record.to_string()]
					.into_iter()
					.collect::<StringRecord>();

				let byte_record = string_record.into_byte_record();

				output_writer
					.write_byte_record(&byte_record)
					.await
					.into_diagnostic()?;
			}
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
	/// The folder to output files to
	#[arg(short, long, value_name = "DIRECTORY")]
	output_folder: PathBuf,
}
