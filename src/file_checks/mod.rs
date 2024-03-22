mod error;

use std::path::{Path, PathBuf};

use csv_async::{AsyncReaderBuilder, AsyncWriterBuilder};
use futures::{StreamExt as _, TryFutureExt as _, TryStreamExt as _};
use tokio::fs::File;
use tracing::{event, Level};

pub use self::error::{CheckFileError, CheckFolderError};
use crate::{util::visit, BadDataRecord};

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

pub async fn check_file(input_path: &Path, output_path: &Path) -> Result<(), CheckFileError> {
	assert!(input_path.is_file());
	event!(Level::DEBUG, ?input_path, "checking file");

	let output_path = output_path.join(input_path.file_name().and_then(|s| s.to_str()).unwrap());

	let output_file = File::create(output_path);

	let mut output_writer = AsyncWriterBuilder::new()
		.flexible(false)
		.has_headers(false)
		.create_serializer(output_file.await?);

	let input_file = File::open(input_path);

	let mut input_reader = AsyncReaderBuilder::new()
		.has_headers(false)
		.create_reader(input_file.await?);

	let mut input_stream = input_reader
		.records()
		.try_filter_map(|item| {
			let index = item.get(0).and_then(|s| s.parse::<usize>().ok());
			let barcode = item.get(4).and_then(|s| {
				s[..s.len().checked_sub(1).unwrap_or(s.len())]
					.parse::<u64>()
					.ok()
			});

			futures::future::ok(index.zip(barcode))
		})
		.enumerate();

	let mut checked = Vec::new();

	while let Some((i, result)) = input_stream.next().await {
		let (record_index, record) = result?;
		let Some((_, before)) = checked.get(i.checked_sub(1).unwrap_or_default()).copied() else {
			checked.push((record_index, record));
			continue;
		};

		if record - 1 != before {
			event!(Level::ERROR, ?input_path, %record_index, %record);

			let record = BadDataRecord {
				index: record_index,
				data: record,
			};

			output_writer.serialize(record).await?;
		}

		checked.push((record_index, record));
	}

	Ok(())
}
