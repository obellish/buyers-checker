use std::path::Path;

use miette::{IntoDiagnostic as _, Result};
use tokio::fs;
use tracing_subscriber::{
	fmt::{
		format::{Compact, DefaultFields, Format, Pretty},
		layer, Layer,
	},
	prelude::*,
	EnvFilter,
};

pub async fn setup_tracing<P>(output_folder: P) -> Result<()>
where
	P: AsRef<Path> + Send,
{
	let log_filter_layer = EnvFilter::try_from_default_env()
		.or_else(|_| EnvFilter::try_new("debug"))
		.into_diagnostic()?;

	let log_fmt_layer = setup_console();
	let log_fs_layer = setup_file(output_folder).await?;

	tracing_subscriber::registry()
		.with(log_filter_layer)
		.with(log_fmt_layer)
		.with(log_fs_layer)
		.try_init()
		.into_diagnostic()?;

	Ok(())
}

fn setup_console<T>() -> Layer<T, Pretty, Format<Pretty>> {
	layer()
		.pretty()
		.with_ansi(true)
		.with_thread_ids(true)
		.with_thread_names(true)
}

async fn setup_file<P, T>(
	output_folder: P,
) -> Result<Layer<T, DefaultFields, Format<Compact>, std::fs::File>>
where
	P: AsRef<Path> + Send,
{
	let output_folder = output_folder.as_ref();
	_ = fs::remove_dir_all(&output_folder).await;

	fs::create_dir_all(output_folder).await.into_diagnostic()?;
	let output_log_file = fs::File::create(output_folder.join("log_output.log"))
		.await
		.into_diagnostic()?
		.into_std()
		.await;

	let log_fs_layer = layer()
		.compact()
		.with_ansi(false)
		.with_writer(output_log_file);

	Ok(log_fs_layer)
}
