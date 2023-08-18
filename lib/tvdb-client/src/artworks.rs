use serde::Deserialize;

macro_rules! artworks {
	(
		$(#[$enum_meta:meta])*
		$vis:vis enum $enum_name:ident {
			$(
				$(#[$meta:meta])*
				$variant:ident = $value:literal,
			)*
		}
	) => {
		#[repr(u8)]
		$(#[$enum_meta])*
		$vis enum $enum_name {
			Unknown = 0,
			$(
				$(#[$meta])*
				$variant = $value,
			)*
		}

		impl From<u8> for $enum_name {
			fn from(value: u8) -> Self {
				match value {
					$(
						$value => Self::$variant,
					)*
					_ => Self::Unknown,
				}
			}
		}

		impl From<$enum_name> for u8 {
			#[inline(always)]
			fn from(value: $enum_name) -> Self {
				value as u8
			}
		}
	};
}

artworks! {
	#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
	pub enum ArtworkKind {
		SeriesBanner = 1,
		SeriesPoster = 2,
		SeriesBackground = 3,
		SeriesIcon = 5,
		SeasonBanner = 6,
		SeasonPoster = 7,
		SeasonBackground = 8,
		SeasonIcon = 10,
		Episode16x9Screencap = 11,
		Episode4x3Screencap = 12,
		ActorPhoto = 13,
		MoviePoster = 14,
		MovieBackground = 15,
		MovieBanner = 16,
		MovieIcon = 18,
		CompanyIcon = 19,
		SeriesCinemagraph = 20,
		MovieCinemagraph = 21,
		SeriesClearArt = 22,
		SeriesClearLogo = 23,
		MovieClearArt = 24,
		MovieClearLogo = 25,
		AwardIcon = 26,
		ListPoster = 27,
	}
}

impl PartialEq<u8> for ArtworkKind {
	#[inline(always)]
	fn eq(&self, other: &u8) -> bool {
		u8::from(*self) == *other
	}
}

impl PartialEq<ArtworkKind> for u8 {
	#[inline(always)]
	fn eq(&self, other: &ArtworkKind) -> bool {
		*other == *self
	}
}

impl<'de> Deserialize<'de> for ArtworkKind {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		let value = u8::deserialize(deserializer)?;
		Ok(value.into())
	}
}
