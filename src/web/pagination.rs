use axum::http::Uri;
use indexmap::IndexMap;
use rstml_component::{
	write_html, HtmlAttributeFormatter, HtmlAttributeValue, HtmlComponent, HtmlContent, HtmlFormatter,
};
use serde::Deserialize;
use std::{fmt, ops};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct PageNumber(u64);

impl PageNumber {
	const FIRST: PageNumber = PageNumber(0);

	pub fn index(&self) -> u64 {
		self.0
	}

	pub fn display(&self) -> u64 {
		self.0 + 1
	}
}

impl PartialEq<u64> for PageNumber {
	fn eq(&self, other: &u64) -> bool {
		self.index() == *other
	}
}

impl PartialEq<PageNumber> for u64 {
	fn eq(&self, other: &PageNumber) -> bool {
		*self == other.index()
	}
}

impl PartialOrd<u64> for PageNumber {
	fn partial_cmp(&self, other: &u64) -> Option<std::cmp::Ordering> {
		self.index().partial_cmp(other)
	}
}

impl PartialOrd<PageNumber> for u64 {
	fn partial_cmp(&self, other: &PageNumber) -> Option<std::cmp::Ordering> {
		self.partial_cmp(&other.index())
	}
}

impl ops::Sub<u64> for PageNumber {
	type Output = Self;

	fn sub(self, rhs: u64) -> Self::Output {
		Self(self.0 - rhs)
	}
}

impl ops::Add<u64> for PageNumber {
	type Output = Self;

	fn add(self, rhs: u64) -> Self::Output {
		Self(self.0 + rhs)
	}
}

impl fmt::Display for PageNumber {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "{}", self.display())
	}
}

impl HtmlAttributeValue for PageNumber {
	fn fmt(self, formatter: &mut HtmlAttributeFormatter) -> fmt::Result {
		HtmlAttributeValue::fmt(self.0 + 1, formatter)
	}
}

impl HtmlContent for PageNumber {
	fn fmt(self, formatter: &mut HtmlFormatter) -> fmt::Result {
		HtmlContent::fmt(self.0 + 1, formatter)
	}
}

impl<'de> Deserialize<'de> for PageNumber {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		let v: Option<u64> = Option::deserialize(deserializer)?;
		match v {
			None => Ok(Self(0)),
			Some(0) => Ok(Self(0)),
			Some(v) => Ok(Self(v - 1)),
		}
	}
}

pub struct Page {
	pub page: PageNumber,
	pub query: String,
}

impl Page {
	fn display(&self) -> u64 {
		self.page.display()
	}
}

#[derive(HtmlComponent)]
pub struct Pagination {
	pages: u64,
	first_page: Option<Page>,
	prev_page: Option<Page>,
	current_page: Page,
	next_page: Option<Page>,
	last_page: Option<Page>,
}
impl Pagination {
	pub fn new(pages: u64, page: PageNumber, url: Uri) -> Self {
		let query: IndexMap<&str, Option<String>> =
			serde_urlencoded::from_str(url.query().unwrap_or_default()).unwrap_or_default();

		let query = move |page: PageNumber| {
			let mut query = query.clone();
			if page < 1 {
				query.remove("page");
			} else {
				query.insert("page", Some(page.to_string()));
			}

			let query = serde_urlencoded::to_string(query).unwrap();
			Page { page, query }
		};

		let first_page = (page > 0).then(|| query(PageNumber::FIRST));
		let prev_page = (page > 1).then(|| query(page - 1));
		let current_page = query(page);
		let next_page = (page + 1 < pages).then(|| query(page + 1));
		let last_page = (page + 2 < pages).then(|| query(PageNumber(pages - 1)));

		Self {
			pages,
			first_page,
			prev_page,
			current_page,
			next_page,
			last_page,
		}
	}

	pub fn next_page_href(&self) -> Option<String> {
		self
			.next_page
			.as_ref()
			.map(|page| format!("?{}", page.query))
	}
}

impl HtmlContent for Pagination {
	fn fmt(self, formatter: &mut HtmlFormatter) -> fmt::Result {
		struct PageButton {
			id: &'static str,
			display: u64,
			query: String,
			disabled: bool,
		}

		impl HtmlContent for PageButton {
			fn fmt(self, formatter: &mut HtmlFormatter) -> fmt::Result {
				let disabled_attribute = self.disabled.then_some(("disabled", ()));
				let current_attribute = self.disabled.then_some(("aria-current", "true"));
				let current_text = if self.disabled {
					"Current Page, Page "
				} else {
					"Goto Page "
				};

				write_html!(formatter,
					<li>
						<a
							id=self.id href=("?", self.query)
							class="join-item btn"
							aria-label=(current_text, self.display)
							{disabled_attribute}
							{current_attribute}
						>{self.display}</a>
					</li>
				)
			}
		}

		let first_page = self.first_page.map(|page| PageButton {
			id: "first-page",
			display: page.display(),
			query: page.query,
			disabled: false,
		});

		let prev_page = self.prev_page.map(|page| PageButton {
			id: "prev-page",
			display: page.display(),
			query: page.query,
			disabled: false,
		});

		let current_page = PageButton {
			id: "current-page",
			display: self.current_page.display(),
			query: self.current_page.query,
			disabled: true,
		};

		let next_page = self.next_page.map(|page| PageButton {
			id: "next-page",
			display: page.display(),
			query: page.query,
			disabled: false,
		});

		let last_page = self.last_page.map(|page| PageButton {
			id: "last-page",
			display: page.display(),
			query: page.query,
			disabled: false,
		});

		write_html!(formatter,
			<nav class="flex flex-row justify-center p-4" role="navigation" aria-label="Pagination Navigation" data-pages=self.pages>
				<ul class="join">
					{first_page}
					{prev_page}
					{current_page}
					{next_page}
					{last_page}
				</ul>
			</nav>
		)
	}
}
