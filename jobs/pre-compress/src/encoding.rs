use async_compression::tokio::write::{BrotliEncoder, GzipEncoder, ZlibEncoder, ZstdEncoder};
use core::fmt;
use std::{
	path::Path,
	pin::Pin,
	task::{Context, Poll},
};
use tokio::io;

#[derive(Debug, Clone, Copy)]
pub enum Encoding {
	Deflate,
	Gzip,
	Brotli,
	Zstd,
}

impl fmt::Display for Encoding {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Encoding::Deflate => f.write_str("deflate"),
			Encoding::Gzip => f.write_str("gzip"),
			Encoding::Brotli => f.write_str("brotli"),
			Encoding::Zstd => f.write_str("zstd"),
		}
	}
}

impl Encoding {
	pub fn to_file_extension(self) -> &'static std::ffi::OsStr {
		match self {
			Encoding::Gzip => std::ffi::OsStr::new(".gz"),
			Encoding::Deflate => std::ffi::OsStr::new(".zz"),
			Encoding::Brotli => std::ffi::OsStr::new(".br"),
			Encoding::Zstd => std::ffi::OsStr::new(".zst"),
		}
	}

	pub fn from_file(path: impl AsRef<Path>) -> Option<Encoding> {
		let path = path.as_ref();
		match path.extension() {
			Some(ext) if ext == "gz" => Some(Encoding::Gzip),
			Some(ext) if ext == "zz" => Some(Encoding::Deflate),
			Some(ext) if ext == "br" => Some(Encoding::Brotli),
			Some(ext) if ext == "zst" => Some(Encoding::Zstd),
			_ => None,
		}
	}

	pub fn encoder<W>(&self, writer: W) -> Encoder<W>
	where
		W: io::AsyncWrite,
	{
		match self {
			Encoding::Deflate => ZlibEncoder::with_quality(writer, async_compression::Level::Best).into(),
			Encoding::Gzip => GzipEncoder::with_quality(writer, async_compression::Level::Best).into(),
			Encoding::Brotli => {
				BrotliEncoder::with_quality(writer, async_compression::Level::Best).into()
			}
			Encoding::Zstd => ZstdEncoder::with_quality(writer, async_compression::Level::Best).into(),
		}
	}
}

pub const ENCODINGS: &[Encoding] = &[
	Encoding::Deflate,
	Encoding::Gzip,
	Encoding::Brotli,
	Encoding::Zstd,
];

// #[pin_project(project = EncoderProjection)]
pub enum Encoder<W>
where
	W: io::AsyncWrite,
{
	Deflate(Pin<Box<ZlibEncoder<W>>>),
	Gzip(Pin<Box<GzipEncoder<W>>>),
	Brotli(Pin<Box<BrotliEncoder<W>>>),
	Zstd(Pin<Box<ZstdEncoder<W>>>),
}

impl<W> From<ZlibEncoder<W>> for Encoder<W>
where
	W: io::AsyncWrite,
{
	fn from(encoder: ZlibEncoder<W>) -> Self {
		Self::Deflate(Box::pin(encoder))
	}
}

impl<W> From<GzipEncoder<W>> for Encoder<W>
where
	W: io::AsyncWrite,
{
	fn from(encoder: GzipEncoder<W>) -> Self {
		Self::Gzip(Box::pin(encoder))
	}
}

impl<W> From<BrotliEncoder<W>> for Encoder<W>
where
	W: io::AsyncWrite,
{
	fn from(encoder: BrotliEncoder<W>) -> Self {
		Self::Brotli(Box::pin(encoder))
	}
}

impl<W> From<ZstdEncoder<W>> for Encoder<W>
where
	W: io::AsyncWrite,
{
	fn from(encoder: ZstdEncoder<W>) -> Self {
		Self::Zstd(Box::pin(encoder))
	}
}

impl<W> io::AsyncWrite for Encoder<W>
where
	W: io::AsyncWrite,
{
	fn poll_write(
		self: Pin<&mut Self>,
		cx: &mut Context<'_>,
		buf: &[u8],
	) -> Poll<std::result::Result<usize, std::io::Error>> {
		match self.get_mut() {
			Encoder::Deflate(encoder) => encoder.as_mut().poll_write(cx, buf),
			Encoder::Gzip(encoder) => encoder.as_mut().poll_write(cx, buf),
			Encoder::Brotli(encoder) => encoder.as_mut().poll_write(cx, buf),
			Encoder::Zstd(encoder) => encoder.as_mut().poll_write(cx, buf),
		}
	}

	fn poll_flush(
		self: Pin<&mut Self>,
		cx: &mut Context<'_>,
	) -> Poll<std::result::Result<(), std::io::Error>> {
		match self.get_mut() {
			Encoder::Deflate(encoder) => encoder.as_mut().poll_flush(cx),
			Encoder::Gzip(encoder) => encoder.as_mut().poll_flush(cx),
			Encoder::Brotli(encoder) => encoder.as_mut().poll_flush(cx),
			Encoder::Zstd(encoder) => encoder.as_mut().poll_flush(cx),
		}
	}

	fn poll_shutdown(
		self: Pin<&mut Self>,
		cx: &mut Context<'_>,
	) -> Poll<std::result::Result<(), std::io::Error>> {
		match self.get_mut() {
			Encoder::Deflate(encoder) => encoder.as_mut().poll_shutdown(cx),
			Encoder::Gzip(encoder) => encoder.as_mut().poll_shutdown(cx),
			Encoder::Brotli(encoder) => encoder.as_mut().poll_shutdown(cx),
			Encoder::Zstd(encoder) => encoder.as_mut().poll_shutdown(cx),
		}
	}

	fn poll_write_vectored(
		self: Pin<&mut Self>,
		cx: &mut Context<'_>,
		bufs: &[std::io::IoSlice<'_>],
	) -> Poll<std::result::Result<usize, std::io::Error>> {
		match self.get_mut() {
			Encoder::Deflate(encoder) => encoder.as_mut().poll_write_vectored(cx, bufs),
			Encoder::Gzip(encoder) => encoder.as_mut().poll_write_vectored(cx, bufs),
			Encoder::Brotli(encoder) => encoder.as_mut().poll_write_vectored(cx, bufs),
			Encoder::Zstd(encoder) => encoder.as_mut().poll_write_vectored(cx, bufs),
		}
	}

	fn is_write_vectored(&self) -> bool {
		match self {
			Self::Deflate(encoder) => encoder.is_write_vectored(),
			Self::Gzip(encoder) => encoder.is_write_vectored(),
			Self::Brotli(encoder) => encoder.is_write_vectored(),
			Self::Zstd(encoder) => encoder.is_write_vectored(),
		}
	}
}
