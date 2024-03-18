use std::{
	error::Error as StdError,
	fmt::{Display, Formatter, Result as FmtResult},
	io::Error as IoError,
};

use csv_async::Error as CsvError;
use tokio::task::JoinError;

#[derive(Debug)]
pub enum CheckFolderError {
	Io(IoError),
	CheckFile(CheckFileError),
	Join(JoinError),
}

impl Display for CheckFolderError {
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		match self {
			Self::Io(e) => Display::fmt(e, f),
			Self::CheckFile(e) => Display::fmt(e, f),
			Self::Join(e) => Display::fmt(e, f),
		}
	}
}

impl From<CheckFileError> for CheckFolderError {
	fn from(value: CheckFileError) -> Self {
		Self::CheckFile(value)
	}
}

impl From<IoError> for CheckFolderError {
	fn from(value: IoError) -> Self {
		Self::Io(value)
	}
}

impl From<JoinError> for CheckFolderError {
	fn from(value: JoinError) -> Self {
		Self::Join(value)
	}
}

impl StdError for CheckFolderError {
	fn source(&self) -> Option<&(dyn StdError + 'static)> {
		match self {
			Self::Io(e) => Some(e),
			Self::CheckFile(e) => Some(e),
			Self::Join(e) => Some(e),
		}
	}
}

#[derive(Debug)]
pub enum CheckFileError {
	Io(IoError),
	Csv(CsvError),
}

impl Display for CheckFileError {
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		match self {
			Self::Io(e) => Display::fmt(e, f),
			Self::Csv(e) => Display::fmt(e, f),
		}
	}
}

impl From<CsvError> for CheckFileError {
	fn from(value: CsvError) -> Self {
		Self::Csv(value)
	}
}

impl From<IoError> for CheckFileError {
	fn from(value: IoError) -> Self {
		Self::Io(value)
	}
}

impl StdError for CheckFileError {
	fn source(&self) -> Option<&(dyn StdError + 'static)> {
		match self {
			Self::Io(e) => Some(e),
			Self::Csv(e) => Some(e),
		}
	}
}
