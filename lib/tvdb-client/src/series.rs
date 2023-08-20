use crate::{artworks::ArtworkKind, TvDbClient, TvDbError, TvDbUrl};
use async_trait::async_trait;
use futures::{stream::FuturesUnordered, StreamExt};
use reqwest::Response;
use serde::Deserialize;
use tracing::{error, info, info_span, instrument, Instrument};

#[derive(Deserialize, Debug)]
struct SeriesDto {
	id: u64,
	name: String,
	#[serde(default)]
	overview: Option<String>,
	#[serde(default)]
	image: Option<String>,
	seasons: Vec<SeriesSeasonDto>,
	#[serde(default, deserialize_with = "nullable_vec", alias = "artwork")]
	artworks: Vec<ArtworkDto>,
	translations: TranslationsDto,
}

#[derive(Deserialize, Debug)]
struct ArtworkDto {
	#[serde(rename = "type")]
	kind: ArtworkKind,
	#[serde(default)]
	image: Option<String>,
	// #[serde(default)]
	// thumbnail: Option<String>,
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
	#[serde(default)]
	name: Option<String>,
	#[serde(default)]
	overview: Option<String>,
	#[serde(default)]
	translations: TranslationsDto,
	#[serde(default)]
	image: Option<String>,
	#[serde(default, deserialize_with = "nullable_vec", alias = "artwork")]
	artworks: Vec<ArtworkDto>,
}

#[derive(Deserialize, Default, Debug)]
struct TranslationsDto {
	#[serde(rename = "nameTranslations", default)]
	name_translations: Option<Vec<NameTranslationDto>>,

	#[serde(rename = "overviewTranslations", default)]
	overview_translations: Option<Vec<OverviewTranslationDto>>,
}

#[derive(Deserialize, Debug)]
struct NameTranslationDto {
	language: String,
	name: String,
}

#[derive(Deserialize, Debug)]
struct OverviewTranslationDto {
	language: String,
	overview: String,
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
	pub description: Option<String>,
	pub image: Option<String>,
	pub seasons: Vec<Season>,
}

pub struct Season {
	pub id: u64,
	pub number: u16,
	pub name: Option<String>,
	pub description: Option<String>,
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

	let body_str = season_response.text().await?;
	let season = serde_json::from_str::<ResultDto<SeasonDto>>(&body_str)
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

	let overview = season
		.translations
		.overview_translations
		.into_iter()
		.flatten()
		.find_map(|t| {
			if t.language == "eng" {
				Some(t.overview)
			} else {
				None
			}
		})
		.or(season.overview);

	let image = get_image(season.image, season.artworks, ArtworkKind::SeasonPoster);
	Ok(Season {
		id,
		number,
		name,
		description: overview,
		image,
	})
}

fn get_image(
	image: Option<String>,
	artworks: Vec<ArtworkDto>,
	kind: ArtworkKind,
) -> Option<String> {
	let mut artworks = artworks
		.into_iter()
		.filter(|a| a.image.is_some())
		.filter(|a| a.kind == kind);

	// if the "item.image" is included in the set, use it, else pick the one with the highest score
	let mut max_score = match artworks.next() {
		None => return None,
		Some(artwork) if artwork.image == image => return image,
		Some(artwork) => artwork,
	};

	for artwork in artworks {
		if artwork.score > max_score.score {
			max_score = artwork;
		}
	}

	max_score.image
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
	let image = get_image(series.image, series.artworks, ArtworkKind::SeriesPoster);
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

	let overview = series
		.translations
		.overview_translations
		.into_iter()
		.flatten()
		.find_map(|t| {
			if t.language == "eng" {
				Some(t.overview)
			} else {
				None
			}
		})
		.or(series.overview);

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
		description: overview,
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
