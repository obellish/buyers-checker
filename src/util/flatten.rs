use std::{
	io,
	path::{Path, PathBuf},
};

use futures::{stream, Stream, StreamExt as _, TryStreamExt as _};
use tokio::fs::{self, DirEntry};
use tokio_stream::wrappers::ReadDirStream;

async fn one_level<P>(path: P, to_visit: &mut Vec<PathBuf>) -> io::Result<Vec<DirEntry>>
where
	P: AsRef<Path> + Send,
{
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
			Err(e) => stream::once(futures::future::err(e)).right_stream(),
		};

		Some((file_stream, to_visit))
	})
	.flatten()
}
