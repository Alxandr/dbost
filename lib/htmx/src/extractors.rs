//! Axum extractors for htmx headers.

use crate::headers::request as req;
use axum::{extract::FromRequestParts, http::request::Parts};

/// The `HX-Boosted` header.
///
/// This is set when a request is made from an element where its parent has the
/// `hx-boost` attribute set to `true`.
///
/// This extractor will always return a value. If the header is not present, it
/// will return `false`.
///
/// See <https://htmx.org/attributes/hx-boost/> for more information.
#[derive(Debug, Clone, Copy)]
pub struct HxBoosted(pub bool);

#[axum::async_trait]
impl<S> FromRequestParts<S> for HxBoosted
where
	S: Send + Sync,
{
	type Rejection = std::convert::Infallible;

	async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
		if parts.headers.contains_key(&req::HX_BOOSTED) {
			return Ok(HxBoosted(true));
		} else {
			return Ok(HxBoosted(false));
		}
	}
}

/// The `HX-Current-Url` header.
///
/// This is set on every request made by htmx itself. As its name implies, it
/// just contains the current url.
///
/// This extractor will always return a value. If the header is not present, it
/// will return `None`.
#[derive(Debug, Clone)]
pub struct HxCurrentUrl(pub Option<String>);

#[axum::async_trait]
impl<S> FromRequestParts<S> for HxCurrentUrl
where
	S: Send + Sync,
{
	type Rejection = std::convert::Infallible;

	async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
		if let Some(url) = parts.headers.get(&req::HX_CURRENT_URL) {
			if let Ok(url) = url.to_str() {
				return Ok(HxCurrentUrl(Some(url.to_string())));
			}
		}

		return Ok(HxCurrentUrl(None));
	}
}

/// The `HX-History-Restore-Request` header.
///
/// This extractor will always return a value. If the header is not present,
/// it will return `false`.
#[derive(Debug, Clone, Copy)]
pub struct HxHistoryRestoreRequest(pub bool);

#[axum::async_trait]
impl<S> FromRequestParts<S> for HxHistoryRestoreRequest
where
	S: Send + Sync,
{
	type Rejection = std::convert::Infallible;

	async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
		if parts.headers.contains_key(&req::HX_HISTORY_RESTORE_REQUEST) {
			return Ok(HxHistoryRestoreRequest(true));
		} else {
			return Ok(HxHistoryRestoreRequest(false));
		}
	}
}

/// The `HX-Prompt` header.
///
/// This is set when a request is made from an element that has the `hx-prompt`
/// attribute set. The value will contain the string input by the user.
///
/// This extractor will always return a value. If the header is not present, it
/// will return `None`.
#[derive(Debug, Clone)]
pub struct HxPrompt(pub Option<String>);

#[axum::async_trait]
impl<S> FromRequestParts<S> for HxPrompt
where
	S: Send + Sync,
{
	type Rejection = std::convert::Infallible;

	async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
		if let Some(prompt) = parts.headers.get(&req::HX_PROMPT) {
			if let Ok(prompt) = prompt.to_str() {
				return Ok(HxPrompt(Some(prompt.to_string())));
			}
		}

		return Ok(HxPrompt(None));
	}
}

/// The `HX-Request` header.
///
/// This is set on every request made by htmx itself. It won't be present on
/// requests made manually, or by other libraries.
///
/// This extractor will always return a value. If the header is not present, it
/// will return `false`.
#[derive(Debug, Clone, Copy)]
pub struct HxRequest(pub bool);

#[axum::async_trait]
impl<S> FromRequestParts<S> for HxRequest
where
	S: Send + Sync,
{
	type Rejection = std::convert::Infallible;

	async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
		if parts.headers.contains_key(&req::HX_REQUEST) {
			return Ok(HxRequest(true));
		} else {
			return Ok(HxRequest(false));
		}
	}
}

/// The `HX-Target` header.
///
/// This is set when a request is made from an element that has the `hx-target`
/// attribute set. The value will contain the target element's id. If the
/// id does not exist on the page, the value will be None.
///
/// This extractor will always return a value. If the header is not present, it
/// will return `None`.
#[derive(Debug, Clone)]
pub struct HxTarget(pub Option<String>);

#[axum::async_trait]
impl<S> FromRequestParts<S> for HxTarget
where
	S: Send + Sync,
{
	type Rejection = std::convert::Infallible;

	async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
		if let Some(target) = parts.headers.get(&req::HX_TARGET) {
			if let Ok(target) = target.to_str() {
				return Ok(HxTarget(Some(target.to_string())));
			}
		}

		return Ok(HxTarget(None));
	}
}

/// The `HX-Trigger-Name` header.
///
/// This is set when a request is made from an element that has the `hx-trigger`
/// attribute set. The value will contain the trigger element's name. If the
/// name does not exist on the page, the value will be None.
///
/// This extractor will always return a value. If the header is not present, it
/// will return `None`.
#[derive(Debug, Clone)]
pub struct HxTriggerName(pub Option<String>);

#[axum::async_trait]
impl<S> FromRequestParts<S> for HxTriggerName
where
	S: Send + Sync,
{
	type Rejection = std::convert::Infallible;

	async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
		if let Some(trigger_name) = parts.headers.get(&req::HX_TRIGGER_NAME) {
			if let Ok(trigger_name) = trigger_name.to_str() {
				return Ok(HxTriggerName(Some(trigger_name.to_string())));
			}
		}

		return Ok(HxTriggerName(None));
	}
}

/// The `HX-Trigger` header.
///
/// This is set when a request is made from an element that has the `hx-trigger`
/// attribute set. The value will contain the trigger element's id. If the
/// id does not exist on the page, the value will be None.
///
/// This extractor will always return a value. If the header is not present, it
/// will return `None`.
#[derive(Debug, Clone)]
pub struct HxTrigger(pub Option<String>);

#[axum::async_trait]
impl<S> FromRequestParts<S> for HxTrigger
where
	S: Send + Sync,
{
	type Rejection = std::convert::Infallible;

	async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
		if let Some(trigger) = parts.headers.get(&req::HX_TRIGGER) {
			if let Ok(trigger) = trigger.to_str() {
				return Ok(HxTrigger(Some(trigger.to_string())));
			}
		}

		return Ok(HxTrigger(None));
	}
}

pub struct HtmxRequestInfo {
	pub boosted: bool,
	pub current_url: Option<String>,
	pub history_restore_request: bool,
	pub prompt: Option<String>,
	pub target: Option<String>,
	pub trigger_name: Option<String>,
	pub trigger: Option<String>,
}

pub struct HxRequestInfo(pub Option<HtmxRequestInfo>);

#[axum::async_trait]
impl<S> FromRequestParts<S> for HxRequestInfo
where
	S: Send + Sync,
{
	type Rejection = std::convert::Infallible;

	async fn from_request_parts(parts: &mut Parts, s: &S) -> Result<Self, Self::Rejection> {
		let HxRequest(hx_request) = HxRequest::from_request_parts(parts, s).await?;
		if !hx_request {
			return Ok(HxRequestInfo(None));
		}

		let HxBoosted(boosted) = HxBoosted::from_request_parts(parts, s).await?;
		let HxCurrentUrl(current_url) = HxCurrentUrl::from_request_parts(parts, s).await?;
		let HxHistoryRestoreRequest(history_restore_request) =
			HxHistoryRestoreRequest::from_request_parts(parts, s).await?;
		let HxPrompt(prompt) = HxPrompt::from_request_parts(parts, s).await?;
		let HxTarget(target) = HxTarget::from_request_parts(parts, s).await?;
		let HxTriggerName(trigger_name) = HxTriggerName::from_request_parts(parts, s).await?;
		let HxTrigger(trigger) = HxTrigger::from_request_parts(parts, s).await?;

		Ok(HxRequestInfo(Some(HtmxRequestInfo {
			boosted,
			current_url,
			history_restore_request,
			prompt,
			target,
			trigger_name,
			trigger,
		})))
	}
}
