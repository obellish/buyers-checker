mod error;

use std::path::{Path, PathBuf};

use csv_async::{AsyncReaderBuilder, AsyncWriterBuilder, StringRecord};
use futures::{
	future::{BoxFuture, FutureExt},
	TryStreamExt as _,
};
use tokio::fs::File;
use tracing::{event, Level};

pub use self::error::{CheckFileError, CheckFolderError};

pub async fn check_directory(path: PathBuf, output_path: PathBuf) -> Result<(), CheckFolderError> {
	event!(Level::INFO, ?path, "checking directory");
	assert!(path.is_dir());

	let mut stream = std::pin::pin!(flatten_dir::visit(path));
	let mut futures = Vec::new();

	while let Some(entry) = stream.try_next().await? {
		let path = entry.path();
		let output_path = output_path.clone();
		futures.push(tokio::spawn(async move {
			check_file(&path, &output_path).await
		}));
	}

	futures::future::try_join_all(futures)
		.await?
		.into_iter()
		.collect::<Result<(), CheckFileError>>()?;

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

mod flatten_dir {
	use std::{io, path::PathBuf};

	use futures::{stream, Stream, StreamExt as _, TryStreamExt as _};
	use tokio::fs::{self, DirEntry};
	use tokio_stream::wrappers::ReadDirStream;

	async fn one_level(path: PathBuf, to_visit: &mut Vec<PathBuf>) -> io::Result<Vec<DirEntry>> {
		let mut dir = ReadDirStream::new(fs::read_dir(path).await?);
		let mut files = Vec::new();

		while let Some(child) = dir.try_next().await? {
			if child.metadata().await?.is_dir() {
				to_visit.push(child.path());
			} else {
				files.push(child);
			}
		}

		Ok(files)
	}

	pub fn visit<P>(path: P) -> impl Stream<Item = Result<DirEntry, io::Error>>
	where
		P: Into<PathBuf>,
	{
		stream::unfold(vec![path.into()], |mut to_visit| async {
			let path = to_visit.pop()?;
			let file_stream = match one_level(path, &mut to_visit).await {
				Ok(files) => stream::iter(files).map(Ok).left_stream(),
				Err(e) => stream::once(async { Err(e) }).right_stream(),
			};

			Some((file_stream, to_visit))
		})
		.flatten()
	}
}
