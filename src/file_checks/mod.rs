mod error;

use std::path::{Path, PathBuf};

use csv_async::{AsyncReaderBuilder, AsyncWriterBuilder, StringRecord};
use futures::{
	future::{BoxFuture, FutureExt},
	TryFutureExt, TryStreamExt as _,
};
use tokio::fs::File;
use tracing::{event, Level};

pub use self::error::{CheckFileError, CheckFolderError};
use crate::util::visit;

pub async fn check_directory(path: PathBuf, output_path: PathBuf) -> Result<(), CheckFolderError> {
	event!(Level::INFO, ?path, "checking directory");
	assert!(path.is_dir());

	let mut stream = std::pin::pin!(visit(path));
	let mut futures = Vec::new();

	while let Some(entry) = stream.try_next().await? {
		let path = entry.path();
		let output_path = output_path.clone();
		futures.push(tokio::spawn(async move {
			check_file(&path, &output_path).await
		}));
	}

	futures::future::try_join_all(futures)
		.map_ok(|values| values.into_iter().collect::<Result<(), CheckFileError>>())
		.await??;

	Ok(())
}

pub fn check_file<'a>(
	input_path: &'a Path,
	output_path: &'a Path,
) -> BoxFuture<'a, Result<(), CheckFileError>> {
	assert!(input_path.is_file());
	event!(Level::DEBUG, ?input_path, "checking file");
	async move {
		let file = File::open(&input_path).await?;

		let mut reader = AsyncReaderBuilder::new()
			.has_headers(false)
			.create_reader(file);

		let records = reader
			.records()
			.try_filter_map(|item| {
				let index = item.get(0).and_then(|s| s.parse::<usize>().ok());
				let barcode = item.get(4).and_then(|s| {
					let mut out = s.to_owned();
					out.pop();
					out.parse::<u64>().ok()
				});

				futures::future::ok(index.zip(barcode))
			})
			.try_collect::<Vec<_>>()
			.await?;

		let output_path = output_path.join(
			input_path
				.file_name()
				.and_then(|s| s.to_str())
				.unwrap_or("FAIL.txt"),
		);

		let output_file = File::create(output_path);

		let mut output_writer = AsyncWriterBuilder::new()
			.flexible(false)
			.has_headers(false)
			.create_writer(output_file.await?);

		for (i, (record_index, record)) in records.iter().copied().enumerate().skip(1) {
			let Some((_, before)) = records.get(i - 1).copied() else {
				continue;
			};

			if record - 1 != before {
				event!(Level::ERROR, ?input_path, %record_index, %record);

				let string_record = [record_index.to_string(), record.to_string()]
					.into_iter()
					.collect::<StringRecord>();

				let byte_record = string_record.into_byte_record();

				output_writer.write_byte_record(&byte_record).await?;
			}
		}

		Ok(())
	}
	.boxed()
}
