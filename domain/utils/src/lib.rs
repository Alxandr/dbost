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
