mod error;

use std::{
	path::{Path, PathBuf},
	pin::pin,
};

use csv_async::AsyncReaderBuilder;
use futures::{StreamExt as _, TryStreamExt as _};
use rust_xlsxwriter::{Workbook, Worksheet};
use tokio::{
	fs::{DirEntry, File},
	io::AsyncWriteExt,
};

pub use self::error::ExcelOutputError;
use crate::util::visit;

pub async fn collect_csv_into_workbook(output_folder: &Path) -> Result<(), ExcelOutputError> {
	let all_csv_files: Vec<_> = collect_all_file_paths(output_folder).await?;

	let mut workbook = Workbook::new();

	for path in all_csv_files {
		let mut sheet = Worksheet::new();

		let mut file_name = path
			.file_name()
			.and_then(|s| s.to_str())
			.unwrap()
			.to_owned();

		file_name.truncate(file_name.len() - 4);

		if file_name.len() > 31 {
			file_name.truncate(31);
		}

		sheet.set_name(file_name)?;

		let file = File::open(path).await?;

		let mut input_reader = AsyncReaderBuilder::new()
			.has_headers(false)
			.flexible(false)
			.create_reader(file);

		let mut input_stream = input_reader.records().enumerate();

		while let Some((i, result)) = input_stream.next().await {
			let record = result?;
			let i = i as u32;

			sheet.write(i, 0, record.get(0).unwrap())?;
			sheet.write(i, 1, record.get(1).unwrap())?;
		}

		workbook.push_worksheet(sheet);
	}

	workbook.read_only_recommended();

	let output_buffer = workbook.save_to_buffer()?;

	let mut output_file = File::create(output_folder.join("output.xlsx")).await?;

	output_file.write_all(&output_buffer).await?;

	Ok(())
}

async fn collect_all_file_paths<C, P>(output_folder: P) -> Result<C, ExcelOutputError>
where
	C: FromIterator<PathBuf>,
	P: AsRef<Path> + Send,
{
	let input_stream = visit(output_folder);

	let temp_output: Vec<_> = input_stream.try_collect().await?;

	Ok(temp_output
		.into_iter()
		.filter_map(|dir| {
			let path = dir.path();
			let extension = path.extension().and_then(|s| s.to_str());

			if extension == Some("csv") {
				Some(path)
			} else {
				None
			}
		})
		.collect())
}
