use crate::{TvDbClient, TvDbError, TvDbUrl};
use async_trait::async_trait;
use futures::{stream::FuturesUnordered, StreamExt};
use reqwest::Response;
use serde::Deserialize;
use tracing::{error, info, info_span, instrument, Instrument};

#[derive(Deserialize)]
struct SeriesDto {
	id: u64,
	name: String,
	seasons: Vec<SeriesSeasonDto>,
	translations: TranslationsDto,
}

#[derive(Deserialize)]
struct SeriesSeasonDto {
	id: u64,
	#[serde(rename = "type")]
	season_type: SeasonTypeDto,
	number: u16,
}

#[derive(Deserialize)]
struct SeasonDto {
	// id: u64,
	#[serde(default = "Default::default")]
	name: Option<String>,
	translations: TranslationsDto,
}

#[derive(Deserialize)]
struct TranslationsDto {
	#[serde(rename = "nameTranslations")]
	name_translations: Vec<TranslationDto>,
}

#[derive(Deserialize)]
struct TranslationDto {
	language: String,
	name: String,
}

#[derive(Deserialize)]
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
	pub seasons: Vec<Season>,
}

pub struct Season {
	pub id: u64,
	pub number: u16,
	pub name: Option<String>,
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

	let season = season_response.json::<ResultDto<SeasonDto>>().await?.data;
	let name = season
		.translations
		.name_translations
		.into_iter()
		.find_map(|t| {
			if t.language == "eng" {
				Some(t.name)
			} else {
				None
			}
		})
		.or(season.name);

	Ok(Season { id, number, name })
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
	let name = series
		.translations
		.name_translations
		.into_iter()
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

	Ok(Some(Series { id, name, seasons }))
}
