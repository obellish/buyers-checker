use std::{
	error::Error as StdError,
	fmt::{Display, Formatter, Result as FmtResult},
	io::Error as IoError,
};

use csv_async::Error as CsvError;
use rust_xlsxwriter::XlsxError;

#[derive(Debug)]
pub enum ExcelOutputError {
	Io(IoError),
	Csv(CsvError),
	Xlsx(XlsxError),
	NoFileName,
}

impl Display for ExcelOutputError {
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		match self {
			Self::Io(e) => Display::fmt(e, f),
			Self::Csv(e) => Display::fmt(e, f),
			Self::Xlsx(e) => Display::fmt(e, f),
			Self::NoFileName => f.write_str("no file name was present in the path"),
		}
	}
}

impl From<IoError> for ExcelOutputError {
	fn from(value: IoError) -> Self {
		Self::Io(value)
	}
}

impl From<CsvError> for ExcelOutputError {
	fn from(value: CsvError) -> Self {
		Self::Csv(value)
	}
}

impl From<XlsxError> for ExcelOutputError {
	fn from(value: XlsxError) -> Self {
		Self::Xlsx(value)
	}
}

impl StdError for ExcelOutputError {
	fn source(&self) -> Option<&(dyn StdError + 'static)> {
		match self {
			Self::Io(e) => Some(e),
			Self::Csv(e) => Some(e),
			Self::Xlsx(e) => Some(e),
			Self::NoFileName => None,
		}
	}
}
