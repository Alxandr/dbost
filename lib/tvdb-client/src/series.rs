use crate::{TvDbClient, TvDbError, TvDbUrl};
use async_trait::async_trait;
use futures::{stream::FuturesUnordered, StreamExt};
use itertools::Itertools;
use reqwest::Response;
use serde::Deserialize;
use tracing::{error, info, info_span, instrument, Instrument};

#[derive(Deserialize, Debug)]
struct SeriesDto {
	id: u64,
	name: String,
	seasons: Vec<SeriesSeasonDto>,
	#[serde(default)]
	image: Option<String>,
	#[serde(default, deserialize_with = "nullable_vec")]
	artworks: Vec<ArtworkDto>,
	translations: TranslationsDto,
}

#[derive(Deserialize, Debug)]
struct ArtworkDto {
	#[serde(rename = "type")]
	ty: i32,
	#[serde(default)]
	image: Option<String>,
	#[serde(default)]
	thumbnail: Option<String>,
	// #[serde(default)]
	// language: Option<String>,
	#[serde(default)]
	score: u32,
}

#[derive(Deserialize, Debug)]
struct SeriesSeasonDto {
	id: u64,
	#[serde(rename = "type")]
	season_type: SeasonTypeDto,
	number: u16,
}

#[derive(Deserialize, Debug)]
struct SeasonDto {
	// id: u64,
	#[serde(default)]
	name: Option<String>,
	#[serde(default)]
	translations: TranslationsDto,
	#[serde(default)]
	image: Option<String>,
	#[serde(default, deserialize_with = "nullable_vec")]
	artworks: Vec<ArtworkDto>,
}

#[derive(Deserialize, Default, Debug)]
struct TranslationsDto {
	#[serde(rename = "nameTranslations", default)]
	name_translations: Option<Vec<TranslationDto>>,
}

#[derive(Deserialize, Debug)]
struct TranslationDto {
	language: String,
	name: String,
}

#[derive(Deserialize, Debug)]
struct SeasonTypeDto {
	// id: u64,
	// name: String,
	#[serde(rename = "type")]
	ty: String,
}

#[derive(Deserialize)]
struct ResultDto<T> {
	data: T,
}

pub struct Series {
	pub id: u64,
	pub name: String,
	pub image: Option<String>,
	pub seasons: Vec<Season>,
}

pub struct Season {
	pub id: u64,
	pub number: u16,
	pub name: Option<String>,
	pub image: Option<String>,
}

async fn check_respons_status(response: Response) -> Result<Response, TvDbError> {
	match response.error_for_status_ref() {
		Ok(_) => Ok(response),
		Err(e) => {
			let status = response.status();
			let body = response.text().await.unwrap_or_default();
			error!(status = %status, "error response: {body}");
			Err(e.into())
		}
	}
}

#[async_trait]
trait ResponseExt {
	async fn if_ok(self) -> Result<Response, TvDbError>;
}

#[async_trait]
impl ResponseExt for Response {
	async fn if_ok(self) -> Result<Response, TvDbError> {
		check_respons_status(self).await
	}
}

async fn get_season(season: SeriesSeasonDto, client: &TvDbClient) -> Result<Season, TvDbError> {
	let id = season.id;
	let number = season.number;

	let season_response = client
		.client
		.get(TvDbUrl::Season(id).into_url())
		.send()
		.await?
		.if_ok()
		.await?;

	let season = season_response
		.json::<ResultDto<SeasonDto>>()
		.await
		.map_err(|e| {
			error!(error = %e, season = id, "failed to parse season response");
			e
		})?
		.data;

	let name = season
		.translations
		.name_translations
		.into_iter()
		.flatten()
		.find_map(|t| {
			if t.language == "eng" {
				Some(t.name)
			} else {
				None
			}
		})
		.or(season.name);

	let image = get_image(season.image, season.artworks);
	Ok(Season {
		id,
		number,
		name,
		image,
	})
}

fn get_image(image: Option<String>, artworks: Vec<ArtworkDto>) -> Option<String> {
	if let Some(image) = image {
		return Some(image);
	}

	let mut artworks = artworks
		.into_iter()
		.filter(|a| matches!(a.ty, 2 | 7))
		.sorted_by_key(|a| a.score)
		.collect_vec();

	artworks
		.iter()
		.enumerate()
		.find(|(_, a)| a.thumbnail.is_some())
		.or_else(|| artworks.iter().enumerate().find(|(_, a)| a.image.is_some()))
		.map(|(i, _)| i)
		.map(|i| artworks.remove(i))
		.and_then(|a| a.image.or(a.thumbnail))
}

#[instrument(skip(client))]
pub(crate) async fn get_series(id: u64, client: &TvDbClient) -> Result<Option<Series>, TvDbError> {
	let url = TvDbUrl::Series(id).into_url();
	info!(url = %url, id = %id, "fetching tmdb series");
	let response = client.client.get(url).send().await?;

	let response = match response.status() {
		reqwest::StatusCode::NOT_FOUND => return Ok(None),
		_ => response.if_ok().await?,
	};

	let series = response.json::<ResultDto<SeriesDto>>().await?.data;
	let id = series.id;
	let image = get_image(series.image, series.artworks);
	let name = series
		.translations
		.name_translations
		.into_iter()
		.flatten()
		.find_map(|t| {
			if t.language == "eng" {
				Some(t.name)
			} else {
				None
			}
		})
		.unwrap_or(series.name);

	let mut seasons = Vec::with_capacity(series.seasons.len());
	let mut futures_unordered = FuturesUnordered::new();
	for season in series
		.seasons
		.into_iter()
		.filter(|s| s.season_type.ty == "official")
	{
		let span = info_span!(
			"fetch season",
			series.id = id,
			series.name = %name,
			season.id = season.id,
			season.number = season.number);
		futures_unordered.push(get_season(season, client).instrument(span));
	}

	while let Some(season) = futures_unordered.next().await {
		seasons.push(season?);
	}

	seasons.sort_by(|l, r| l.number.cmp(&r.number));

	Ok(Some(Series {
		id,
		name,
		seasons,
		image,
	}))
}

fn nullable_vec<'de, D, T>(deserializer: D) -> Result<Vec<T>, D::Error>
where
	D: serde::Deserializer<'de>,
	T: serde::Deserialize<'de>,
{
	let opt = Option::deserialize(deserializer)?;
	Ok(opt.unwrap_or_default())
}
