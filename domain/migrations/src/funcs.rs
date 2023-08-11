use sea_orm_migration::prelude::*;

#[derive(Iden, Clone, Copy)]
enum PgTimeFunctions {
	#[iden = "now"]
	Now,

	#[iden = "timezone"]
	Timezone,
}

pub struct PgTimeFunc;

impl PgTimeFunc {
	pub fn now() -> FunctionCall {
		Func::cust(PgTimeFunctions::Now)
	}

	pub fn timezone<Tz, Ts>(zone: Tz, timestamp: Ts) -> FunctionCall
	where
		Tz: Into<SimpleExpr>,
		Ts: Into<SimpleExpr>,
	{
		Func::cust(PgTimeFunctions::Timezone).args([zone.into(), timestamp.into()])
	}

	pub fn utc_now() -> FunctionCall {
		PgTimeFunc::timezone("utc", PgTimeFunc::now())
	}
}
