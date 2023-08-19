mod encoding;

use bytes::{Bytes, BytesMut};
use clap::Parser;
use color_eyre::eyre::{Report, Result, WrapErr};
use encoding::{Encoding, ENCODINGS};
use futures::{stream::FuturesUnordered, FutureExt, StreamExt};
use std::{
	future::Future,
	path::{Path, PathBuf},
	pin::Pin,
	task::{Context, Poll},
	time::SystemTime,
};
use tokio::{
	fs,
	io::{AsyncReadExt, AsyncWriteExt},
};
use tracing::{info, instrument, metadata::LevelFilter, Instrument};
use tracing_forest::ForestLayer;
use tracing_subscriber::{prelude::*, EnvFilter};
use walkdir::{DirEntry, WalkDir};

/// CLI to precompress static assets
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
	/// Directory to create compressed files for
	#[arg(short, long, env = "DIR")]
	dir: PathBuf,
}

#[tokio::main]
async fn main() -> Result<()> {
	color_eyre::install()?;
	tracing_subscriber::registry()
		.with(ForestLayer::default())
		.with(
			EnvFilter::builder()
				.with_default_directive(LevelFilter::INFO.into())
				.from_env_lossy(),
		)
		.init();

	_main().await
}

fn is_precompressed_file(entry: &DirEntry) -> bool {
	Encoding::from_file(entry.path()).is_some()
}

async fn _main() -> Result<()> {
	let args = Args::parse();

	let tasks: FuturesUnordered<_> = WalkDir::new(args.dir)
		.into_iter()
		.filter_entry(|e| !is_precompressed_file(e))
		.filter_map(|e| e.ok())
		.filter(|e| e.file_type().is_file())
		.map(|e| FileEntry::try_from(e).map(precompress).map(owned_task))
		.collect::<Result<_>>()?;

	drain(tasks).await
}

#[instrument(skip_all, fields(
	file.path = %file.path.display(),
))]
async fn precompress(file: FileEntry) -> Result<()> {
	let compressed_entries = ENCODINGS
		.iter()
		.filter_map(|e| {
			let file_extension = e.to_file_extension();
			let new_extension = file
				.path
				.extension()
				.map(|extension| {
					let mut os_string = extension.to_os_string();
					os_string.push(file_extension);
					os_string
				})
				.unwrap_or_else(|| file_extension.to_os_string());

			let compressed_path = file.path.with_extension(new_extension);
			let mtime = compressed_path
				.metadata()
				.and_then(|m| m.modified())
				.unwrap_or(SystemTime::UNIX_EPOCH);

			if mtime >= file.mtime {
				None
			} else {
				Some((*e, compressed_path))
			}
		})
		.collect::<Vec<_>>();

	if compressed_entries.is_empty() {
		info!(
			"skipping '{}', all files are up to date",
			file.path.display()
		);
		return Ok(());
	}

	let content = read_file(&file.path, file.len).await?;
	let tasks: FuturesUnordered<_> = compressed_entries
		.into_iter()
		.map(|(enc, path)| compress_file(enc, path, content.clone()))
		.map(owned_task)
		.collect();

	drain(tasks).await
}

#[instrument(skip_all, fields(
	file.path = %path.display(),
	file.len = %len,
))]
async fn read_file(path: &Path, len: u64) -> Result<Bytes> {
	let mut file = fs::File::open(path)
		.await
		.wrap_err_with(|| format!("open file '{}'", path.display()))?;

	let mut buf = BytesMut::with_capacity(len as usize);
	while file
		.read_buf(&mut buf)
		.await
		.wrap_err_with(|| format!("failed to read file '{}' into buffer", path.display()))?
		> 0
	{
		// keep reading
	}

	debug_assert_eq!(len as usize, buf.len());
	Ok(buf.freeze())
}

#[instrument(skip_all, fields(
	file.path = %path.display(),
	encoding = %enc,
))]
async fn compress_file(enc: Encoding, path: PathBuf, mut content: Bytes) -> Result<()> {
	let file = fs::OpenOptions::new()
		.write(true)
		.create(true)
		.truncate(true)
		.open(&path)
		.await
		.wrap_err_with(|| format!("open file '{}'", path.display()))?;

	let mut writer = enc.encoder(file);
	writer.write_all_buf(&mut content).await.wrap_err_with(|| {
		format!(
			"failed to write compressed file '{}' with {}",
			path.display(),
			enc,
		)
	})?;

	writer.shutdown().await.wrap_err_with(|| {
		format!(
			"failed to flush compressed file '{}' with {}",
			path.display(),
			enc,
		)
	})?;

	Ok(())
}

struct FileEntry {
	path: PathBuf,
	mtime: SystemTime,
	len: u64,
}

impl TryFrom<DirEntry> for FileEntry {
	type Error = Report;

	fn try_from(entry: DirEntry) -> Result<Self> {
		let metadata = entry.metadata().wrap_err_with(|| {
			format!(
				"failed to get metadata for file '{}'",
				entry.path().display()
			)
		})?;

		let path = entry.into_path();
		let mtime = metadata
			.modified()
			.wrap_err_with(|| format!("failed to get modified time for file '{}'", path.display()))?;
		let len = metadata.len();

		Ok(Self { path, mtime, len })
	}
}

async fn drain<F>(mut stream: FuturesUnordered<F>) -> Result<()>
where
	F: Future<Output = Result<()>>,
{
	while let Some(result) = stream.next().await {
		result?;
	}

	Ok(())
}

fn owned_task<F>(future: F) -> OwnedTask
where
	F: Future<Output = Result<()>> + Send + Sync + 'static,
{
	OwnedTask {
		handle: tokio::spawn(future.instrument(tracing::Span::current())),
	}
}

struct OwnedTask {
	handle: tokio::task::JoinHandle<Result<()>>,
}

impl Drop for OwnedTask {
	fn drop(&mut self) {
		self.handle.abort();
	}
}

impl Future for OwnedTask {
	type Output = Result<()>;

	fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
		self
			.get_mut()
			.handle
			.poll_unpin(cx)
			.map(|r| r.unwrap_or_else(|e| Err(e.into())))
	}
}
