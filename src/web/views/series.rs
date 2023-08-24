use crate::web::views::Template;
use axum::response::IntoResponse;
use dbost_entities::{season, series, theme_song};
use dbost_session::Session;
use indexmap::IndexMap;
use rstml_component::{
	write_html, For, HtmlAttributeFormatter, HtmlAttributeValue, HtmlComponent, HtmlContent,
	HtmlFormatter,
};
use rstml_component_axum::Html;
use std::fmt;
use uuid::Uuid;

#[derive(HtmlComponent)]
struct VideoPlayer<'a> {
	video: &'a theme_song::Model,
}

impl<'a> HtmlContent for VideoPlayer<'a> {
	fn fmt(self, f: &mut HtmlFormatter) -> fmt::Result {
		write_html!(f, { ("video: ", self.video.id.to_string()) })
	}
}

#[derive(HtmlComponent)]
struct ThemePanel<'a> {
	target: EditTarget,
	mode: EditMode,
	video: Option<&'a theme_song::Model>,
}

impl<'a> HtmlContent for ThemePanel<'a> {
	fn fmt(self, f: &mut HtmlFormatter) -> fmt::Result {
		match self.video {
			None => write_html!(f,
				<div id=self.target.id()>
					<h3 class="flex text-xl font-bold">
						<span class="flex-1">"Theme Song"</span>
						{|f: &mut HtmlFormatter| match self.mode {
							EditMode::Normal { can_edit } => write_html!(f,
								<EditButton
									target=&self.target
									enabled={can_edit} />
							),
							EditMode::Edit => f.write_content("fancy seeing you here"),
						}}
					</h3>

					<div class="flex rounded-lg aspect-video bg-gradient-to-r from-sky-700/50 to-indigo-700/50">
						<p class="self-center block m-auto fit-content">"Theme song missing"</p>
					</div>
				</div>
			),
			Some(theme) => VideoPlayer { video: theme }.fmt(f),
		}
	}
}

#[derive(HtmlComponent)]
struct EditButton<'a> {
	target: &'a EditTarget,
	enabled: bool,
}

impl<'a> HtmlContent for EditButton<'a> {
	fn fmt(self, f: &mut HtmlFormatter) -> fmt::Result {
		if !self.enabled {
			return Ok(());
		}

		write_html!(f,
			<a
				class="invisible float-right btn btn-ghost btn-sm sm:visible"
				href=self.target.href()
			>"edit"</a>
		)
	}
}

#[derive(HtmlComponent)]
struct SeasonRow<'a> {
	series: &'a series::Model,
	season: &'a season::Model,
}

impl<'a> HtmlContent for SeasonRow<'a> {
	fn fmt(self, f: &mut HtmlFormatter) -> fmt::Result {
		let series_id = self.series.id.to_string();
		let season_id = self.season.id.to_string();
		let season_number_display = format!("Season {:02}", self.season.number);
		let season_name = self
			.season
			.name
			.as_deref()
			.unwrap_or(&*season_number_display);

		write_html!(f,
			<li
				id=(&*series_id, "/season/", &*season_id)
				class="flex flex-col gap-4 p-4 rounded-lg sm:flex-row bg-base-200"
			>
				<picture
					class="self-center flex-none w-full sm:self-start sm:w-56"
				>
					<img
						src=self.season.image.as_deref().or(self.series.image.as_deref())
						class="mx-auto rounded-lg shadow-2xl"
						referrerpolicy="no-referrer"
						alt=(season_name, " thumbnail") />
				</picture>
				<div class="flex-1">
					<h2 class="text-3xl font-bold tooltip" data-tip=&*season_number_display>{season_name}</h2>
					<p class="py-6" hx-disable>{self.season.description.as_deref()}</p>
				</div>
			</li>
		)
	}
}

pub enum SeriesEdit {
	None,
	Series,
	// Season(Uuid),
}

enum EditTarget {
	Series(Uuid),
	// Season(Uuid, Uuid),
}

impl EditTarget {
	fn id(&self) -> impl HtmlAttributeValue + '_ {
		struct IdAttributeValue<'a>(&'a EditTarget);
		impl<'a> HtmlAttributeValue for IdAttributeValue<'a> {
			fn fmt(self, f: &mut HtmlAttributeFormatter) -> fmt::Result {
				match self.0 {
					EditTarget::Series(series) => HtmlAttributeValue::fmt(series.to_string(), f),
				}
			}
		}

		IdAttributeValue(self)
	}

	fn href(&self) -> impl HtmlAttributeValue + '_ {
		struct HrefAttributeValue<'a>(&'a EditTarget);
		impl<'a> HtmlAttributeValue for HrefAttributeValue<'a> {
			fn fmt(self, f: &mut HtmlAttributeFormatter) -> fmt::Result {
				match self.0 {
					EditTarget::Series(series) => {
						HtmlAttributeValue::fmt(("/series/", series.to_string(), "/edit"), f)
					}
				}
			}
		}

		HrefAttributeValue(self)
	}
}

enum EditMode {
	Normal { can_edit: bool },
	Edit,
}

impl EditMode {
	fn series(edit: &SeriesEdit, can_edit: bool) -> Self {
		match edit {
			SeriesEdit::None => Self::Normal { can_edit },
			SeriesEdit::Series => Self::Edit,
		}
	}
}

pub struct SeriesPage<'a> {
	session: &'a Session,
	series: series::Model,
	seasons: Vec<season::Model>,
	themes: IndexMap<Uuid, theme_song::Model>,
	edit: SeriesEdit,
}

impl<'a> SeriesPage<'a> {
	pub fn new(
		session: &'a Session,
		series: series::Model,
		seasons: Vec<season::Model>,
		themes: IndexMap<Uuid, theme_song::Model>,
		edit: SeriesEdit,
	) -> Self {
		Self {
			session,
			series,
			seasons,
			themes,
			edit,
		}
	}

	pub fn into_response(self) -> axum::response::Response {
		Html(self).into_response()
	}
}

impl<'a> HtmlContent for SeriesPage<'a> {
	fn fmt(self, f: &mut HtmlFormatter) -> fmt::Result {
		write_html!(f,
			<Template title=&*self.series.name session=self.session>
				<div class="rounded-lg min-h-72 hero">
					<div class="flex-col hero-content lg:flex-row">
						<picture
							class="flex-none w-full lg:self-start sm:w-96 contain-paint"
							style="view-transition-name: series-image;"
						>
							<img
								src=self.series.image.as_deref()
								class="rounded-lg shadow-2xl"
								referrerpolicy="no-referrer"
								alt=(&*self.series.name, " thumbnail") />
						</picture>
						<div class="flex-1">
							<h1 class="text-5xl font-bold">{&*self.series.name}</h1>
							<p class="py-6" hx-disable>{self.series.description.as_deref()}</p>

							<ThemePanel
								target=EditTarget::Series(self.series.id)
								mode=EditMode::series(&self.edit, self.session.user().is_some())
								video=self.series.theme_song_id.map(|id| &self.themes[&id]) />
						</div>
					</div>
				</div>
				<ul class="mt-20 space-y-8">
					<For items={self.seasons}>
						{ |f, s| {
							write_html!(f,
								<SeasonRow series=&self.series season=&s />
							)
						} }
					</For>
				</ul>
			</Template>
		)
	}
}
