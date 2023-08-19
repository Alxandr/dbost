use std::{collections::HashMap, path::Path, sync::OnceLock};

use serde::Deserialize;
use thiserror::Error;
use tokio::fs;

#[derive(Clone, Copy)]
pub struct BuiltAssets {
	pub css: &'static str,
	pub js: &'static str,
}

static ASSETS: OnceLock<BuiltAssets> = OnceLock::new();

impl BuiltAssets {
	pub async fn init(dir: impl AsRef<Path>) -> Result<(), AssetError> {
		let dir = dir.as_ref();
		let manifest = dir.join("manifest.json");

		let manifest = fs::read_to_string(manifest).await?;
		let entries: HashMap<String, AssetManifestEntry> = serde_json::from_str(&manifest)?;

		let css_path: Box<str> = entries
			.get("main.css")
			.ok_or_else(|| AssetError::MissingAsset("main.css".to_owned()))?
			.file
			.as_str()
			.into();

		let js_path: Box<str> = entries
			.get("main.ts")
			.ok_or_else(|| AssetError::MissingAsset("main.ts".to_owned()))?
			.file
			.as_str()
			.into();

		let css_path = Box::leak(css_path);
		let js_path = Box::leak(js_path);

		let _ = ASSETS.set(BuiltAssets {
			css: css_path,
			js: js_path,
		});

		Ok(())
	}

	pub fn assets() -> &'static Self {
		ASSETS
			.get()
			.unwrap_or_else(|| panic!("assets not initialized"))
	}

	// pub fn css() -> &'static str {
	// 	Self::assets().css
	// }

	// pub fn js() -> &'static str {
	// 	Self::assets().js
	// }
}

#[derive(Debug, Error)]
pub enum AssetError {
	#[error(transparent)]
	Io(#[from] std::io::Error),

	#[error(transparent)]
	Json(#[from] serde_json::Error),

	#[error("missing asset: {0}")]
	MissingAsset(String),
}

#[derive(Deserialize)]
struct AssetManifestEntry {
	file: String,
	// #[serde(rename = "isEntry")]
	// is_entry: bool,
	// src: String,
}
