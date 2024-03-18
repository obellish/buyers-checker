use std::{
	error::Error as StdError,
	fmt::{Display, Formatter, Result as FmtResult},
	io::Error as IoError,
};

use tracing_subscriber::{filter::ParseError, util::TryInitError};

#[derive(Debug)]
pub enum TracingSetupError {
	FileSetup(IoError),
	ParseError(ParseError),
	TryInitError(TryInitError),
}

impl Display for TracingSetupError {
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		match self {
			Self::FileSetup(e) => Display::fmt(e, f),
			Self::ParseError(e) => Display::fmt(e, f),
			Self::TryInitError(e) => Display::fmt(e, f),
		}
	}
}

impl From<IoError> for TracingSetupError {
	fn from(value: IoError) -> Self {
		Self::FileSetup(value)
	}
}

impl From<ParseError> for TracingSetupError {
	fn from(value: ParseError) -> Self {
		Self::ParseError(value)
	}
}

impl From<TryInitError> for TracingSetupError {
	fn from(value: TryInitError) -> Self {
		Self::TryInitError(value)
	}
}

impl StdError for TracingSetupError {
	fn source(&self) -> Option<&(dyn StdError + 'static)> {
		match self {
			Self::FileSetup(e) => Some(e),
			Self::ParseError(e) => Some(e),
			Self::TryInitError(e) => Some(e),
		}
	}
}
