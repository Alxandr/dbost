use sea_orm::ActiveValue;
use time::{OffsetDateTime, PrimitiveDateTime, UtcOffset};

pub trait OffsetDateTimeExt {
	fn into_primitive_utc(self) -> PrimitiveDateTime;
}

impl OffsetDateTimeExt for OffsetDateTime {
	fn into_primitive_utc(self) -> PrimitiveDateTime {
		let value = self.to_offset(UtcOffset::UTC);
		let date = value.date();
		let time = value.time();
		PrimitiveDateTime::new(date, time)
	}
}

pub trait ActiveValueExt<T> {
	fn update(&mut self, value: T);
}

impl<T> ActiveValueExt<T> for sea_orm::ActiveValue<T>
where
	T: Into<sea_orm::Value>,
	for<'a> &'a T: Eq,
{
	fn update(&mut self, value: T) {
		match self {
			Self::Set(v) => *v = value,
			Self::NotSet => *self = Self::Set(value),
			Self::Unchanged(v) => {
				if &*v != &value {
					*self = Self::Set(value);
				}
			}
		}
	}
}

pub struct ActiveVersion;

impl ActiveVersion {
	pub fn now() -> ActiveValue<PrimitiveDateTime> {
		ActiveValue::Set(OffsetDateTime::now_utc().into_primitive_utc())
	}
}
