use crate::web::views::Template;
use axum::response::IntoResponse;
use dbost_session::Session;
use rstml_component::{
	write_html, For, HtmlAttributes, HtmlAttributesFormatter, HtmlComponent, HtmlContent,
	HtmlFormatter,
};
use rstml_component_axum::Html;
use std::{fmt, sync::Arc};
use uuid::Uuid;

// temp - move to rstml_component
struct Attrs<I>(I)
where
	I: IntoIterator,
	<I as IntoIterator>::Item: HtmlAttributes;

impl<I> HtmlAttributes for Attrs<I>
where
	I: IntoIterator,
	<I as IntoIterator>::Item: HtmlAttributes,
{
	fn fmt(self, formatter: &mut HtmlAttributesFormatter) -> fmt::Result {
		for attr in self.0 {
			attr.fmt(formatter)?;
		}

		Ok(())
	}
}

#[derive(HtmlComponent)]
pub struct SeriesCard {
	name: String,
	id: Uuid,
	image: Option<String>,
	season_count: i64,
	next_page_link: Option<Arc<str>>,
}

impl SeriesCard {
	pub fn new(
		name: String,
		id: Uuid,
		image: Option<String>,
		season_count: i64,
		next_page_link: Option<Arc<str>>,
	) -> Self {
		Self {
			name,
			id,
			image,
			season_count,
			next_page_link,
		}
	}
}

impl HtmlContent for SeriesCard {
	fn fmt(self, formatter: &mut HtmlFormatter) -> fmt::Result {
		let next_page_attr = self.next_page_link.as_deref().map(|next_page_link| {
			Attrs([
				("hx-get", next_page_link),
				("hx-trigger", "revealed"),
				("hx-swap", "afterend"),
			])
		});

		let id = self.id.to_string();
		write_html!(formatter,
			<li
				id=("series-card-", &*id)
				class="grid grid-cols-1 row-span-2 gap-0 overflow-hidden shadow-xl grid-rows-series-card rounded-box bg-base-100 series-card contain-paint"
				// style=("view-transition-name: ", "series-", &*id, "-image", ";")
				hx-view-transition-name="series-image"
				hx-ext="transition"
				{next_page_attr}
			>
				<a class="contents" href=("/series/", &*id)>
					<picture
						class="series-image rounded-box"
					>
						<img src=self.image.as_deref() alt="" referrerpolicy="no-referrer" />
					</picture>
					<div class="p-4 text-base bg-base-100/80 series-text">
						<h2 class="card-title text-ellipsis line-clamp-2" hx-disable>{&*self.name}</h2>
						<p>"Seasons: " {self.season_count}</p>
					</div>
				</a>
			</li>
		)
	}
}

pub struct IndexPage<'a, I> {
	session: &'a Session,
	items: I,
}

impl<'a, I> IndexPage<'a, I>
where
	I: IntoIterator<Item = SeriesCard>,
{
	pub fn new(session: &'a Session, items: I) -> Self {
		Self { session, items }
	}

	pub fn into_response(self) -> axum::response::Response {
		Html(self).into_response()
	}

	pub fn into_items_fragment_response(self) -> axum::response::Response {
		Html(Self::items_fragment(self.items)).into_response()
	}

	fn items_fragment(items: I) -> impl HtmlContent {
		For {
			items,
			children: |f, item| item.fmt(f),
		}
	}
}

impl<'a, I> HtmlContent for IndexPage<'a, I>
where
	I: IntoIterator<Item = SeriesCard>,
{
	fn fmt(self, f: &mut HtmlFormatter) -> fmt::Result {
		write_html!(f,
			<Template title="Series" session=self.session>
				<h1 class="mb-8 text-4xl font-bold">Series</h1>

				<ul
					class="grid grid-cols-1 gap-4 auto-rows-cards sm:grid-cols-2 md:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5 2xl:grid-cols-6"
					hx-indicator=".htmx-indicator">
					{Self::items_fragment(self.items)}
				</ul>
				<center>
					<img class="htmx-indicator" width="60" src="/public/img/bars.svg" />
				</center>
			</Template>
		)
	}
}

impl<'a, I> IntoResponse for IndexPage<'a, I>
where
	I: IntoIterator<Item = SeriesCard>,
{
	fn into_response(self) -> axum::response::Response {
		self.into_response()
	}
}
