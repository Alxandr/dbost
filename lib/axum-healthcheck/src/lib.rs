use axum::{http::StatusCode, response::IntoResponse, Json};
use indexmap::IndexMap;
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum HealthStatus<'a> {
	Healthy,
	Degraded(&'a str),
	Unhealthy(&'a str),
}

pub trait IntoHealthStatus<'a> {
	fn into_health_status(self) -> HealthStatus<'a>;
}

impl<'a> IntoHealthStatus<'a> for HealthStatus<'a> {
	fn into_health_status(self) -> HealthStatus<'a> {
		self
	}
}

impl<'a, T> IntoHealthStatus<'a> for Result<T, HealthStatus<'a>> {
	fn into_health_status(self) -> HealthStatus<'a> {
		match self {
			Ok(_) => HealthStatus::Healthy,
			Err(status) => status,
		}
	}
}

#[derive(Default)]
pub struct HealthCheck<'a> {
	checks: IndexMap<&'static str, HealthStatus<'a>>,
}

impl<'a> HealthCheck<'a> {
	pub fn new() -> Self {
		Default::default()
	}

	pub fn add(&mut self, name: &'static str, status: impl IntoHealthStatus<'a>) -> &mut Self {
		self.checks.insert(name, status.into_health_status());
		self
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub enum HealthStatusSeverity {
	#[serde(rename = "healthy")]
	Healthy,
	#[serde(rename = "degraded")]
	Degraded,
	#[serde(rename = "unhealthy")]
	Unhealthy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum HealthStatusErrorSeverity {
	Degraded,
	Unhealthy,
}

impl<'a> HealthStatus<'a> {
	pub fn severity(&self) -> HealthStatusSeverity {
		match self {
			HealthStatus::Healthy => HealthStatusSeverity::Healthy,
			HealthStatus::Degraded(_) => HealthStatusSeverity::Degraded,
			HealthStatus::Unhealthy(_) => HealthStatusSeverity::Unhealthy,
		}
	}
}

pub trait ResultHealthStatusExt<'a>: Sized {
	fn into_health_check(
		self,
		severity: HealthStatusErrorSeverity,
		message: &'a str,
	) -> HealthStatus<'a>;

	fn or_degraded(self, message: &'a str) -> HealthStatus<'a> {
		self.into_health_check(HealthStatusErrorSeverity::Degraded, message)
	}

	fn or_unhealthy(self, message: &'a str) -> HealthStatus<'a> {
		self.into_health_check(HealthStatusErrorSeverity::Unhealthy, message)
	}
}

impl<'a, T, E> ResultHealthStatusExt<'a> for Result<T, E> {
	fn into_health_check(
		self,
		severity: HealthStatusErrorSeverity,
		message: &'a str,
	) -> HealthStatus<'a> {
		match self {
			Ok(_) => HealthStatus::Healthy,
			Err(_) => match severity {
				HealthStatusErrorSeverity::Degraded => HealthStatus::Degraded(message),
				HealthStatusErrorSeverity::Unhealthy => HealthStatus::Unhealthy(message),
			},
		}
	}
}

#[derive(Serialize)]
struct HealthCheckResponseData {
	status: HealthStatusSeverity,
	message: Option<String>,
}

#[derive(Serialize)]
struct HealthCheckResponse {
	status: HealthStatusSeverity,
	checks: IndexMap<&'static str, HealthCheckResponseData>,
}

impl<'a, 'b> From<&'b HealthCheck<'a>> for HealthCheckResponse {
	fn from(check: &'b HealthCheck<'a>) -> Self {
		let mut status = HealthStatusSeverity::Healthy;
		let mut checks = IndexMap::new();

		for (name, check) in &check.checks {
			let data = match check {
				HealthStatus::Healthy => HealthCheckResponseData {
					status: HealthStatusSeverity::Healthy,
					message: None,
				},
				HealthStatus::Degraded(message) => HealthCheckResponseData {
					status: HealthStatusSeverity::Degraded,
					message: Some((*message).to_owned()),
				},
				HealthStatus::Unhealthy(message) => HealthCheckResponseData {
					status: HealthStatusSeverity::Unhealthy,
					message: Some((*message).to_owned()),
				},
			};

			if data.status > status {
				status = data.status;
			}

			checks.insert(*name, data);
		}

		Self { status, checks }
	}
}

impl IntoResponse for HealthCheckResponse {
	fn into_response(self) -> axum::response::Response {
		let status_code = match self.status {
			HealthStatusSeverity::Healthy => StatusCode::OK,
			HealthStatusSeverity::Degraded => StatusCode::OK,
			HealthStatusSeverity::Unhealthy => StatusCode::SERVICE_UNAVAILABLE,
		};

		(status_code, Json(self)).into_response()
	}
}

impl<'a> HealthCheck<'a> {
	pub fn into_response(&self) -> axum::response::Response {
		HealthCheckResponse::from(self).into_response()
	}
}
