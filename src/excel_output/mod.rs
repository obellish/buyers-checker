mod error;

use std::{
	path::{Path, PathBuf},
	pin::pin,
};

use csv_async::AsyncReaderBuilder;
use futures::{Stream, StreamExt as _, TryStreamExt as _};
use rust_xlsxwriter::{Workbook, Worksheet};
use tokio::{fs::File, io::AsyncWriteExt as _};

pub use self::error::ExcelOutputError;
use crate::{util::visit, BadDataRecord};

pub async fn collect_csv_into_workbook(output_folder: &Path) -> Result<(), ExcelOutputError> {
	let mut all_csv_files_stream = pin!(collect_all_file_paths(output_folder));

	let mut workbook = Workbook::new();

	while let Some(path) = all_csv_files_stream.try_next().await? {
		let mut sheet = Worksheet::new();

		let mut file_name = path
			.file_name()
			.and_then(|s| s.to_str())
			.ok_or(ExcelOutputError::NoFileName)?
			.to_owned();

		file_name.truncate(file_name.len() - 4);

		if file_name.len() > 31 {
			file_name.truncate(31);
		}

		sheet.set_name(file_name)?;

		let file = File::open(path);

		let mut input_reader = AsyncReaderBuilder::new()
			.has_headers(false)
			.flexible(false)
			.create_deserializer(file.await?);

		let mut input_stream = input_reader.deserialize::<BadDataRecord>().enumerate();

		while let Some((i, result)) = input_stream.next().await {
			let record = result?;
			let i = i as u32;

			sheet.write_string(i, 0, record.index.to_string())?;
			sheet.write_string(i, 1, record.data.to_string())?;
		}

		workbook.push_worksheet(sheet);
	}

	workbook.read_only_recommended();

	let output_buffer = workbook.save_to_buffer()?;

	let mut output_file = File::create(output_folder.join("output.xlsx")).await?;

	output_file.write_all(&output_buffer).await?;

	Ok(())
}

fn collect_all_file_paths<P>(
	output_folder: P,
) -> impl Stream<Item = Result<PathBuf, std::io::Error>>
where
	P: AsRef<Path> + Send,
{
	let input_stream = visit(output_folder);

	input_stream.try_filter_map(|item| {
		let path = item.path();
		let extension = path.extension().and_then(|s| s.to_str());

		futures::future::ok(if extension == Some("csv") {
			Some(path)
		} else {
			None
		})
	})
}
