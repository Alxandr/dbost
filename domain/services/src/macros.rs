macro_rules! define_service {
	(
		$(#[$name_meta:meta])*
		$vis:vis struct $name:ident {
			$(
				$(#[$dep_meta:meta])*
				$dep_vis:vis $dep:ident: $dep_ty:ty
			),+$(,)?
		}
	) => {
		$(#[$name_meta])*
		$vis struct $name {
			$(
				$(#[$dep_meta])*
				$dep_vis $dep: $dep_ty,
			)+
		}

		#[automatically_derived]
		impl<S> ::axum::extract::FromRef<S> for $name
		where
			$($dep_ty: ::axum::extract::FromRef<S>,)+
		{
			fn from_ref(input: &S) -> Self {
				Self {
					$($dep: <$dep_ty as ::axum::extract::FromRef<S>>::from_ref(input),)+
				}
			}
		}

		#[automatically_derived]
		#[::async_trait::async_trait]
		impl<S> ::axum::extract::FromRequestParts<S> for $name
		where
			$($dep_ty: ::axum::extract::FromRef<S>,)+
			S: Sync,
		{
			type Rejection = ::std::convert::Infallible;

			async fn from_request_parts(
				_parts: &mut ::axum::http::request::Parts,
				state: &S,
			) -> Result<Self, Self::Rejection> {
				Ok(<$name as ::axum::extract::FromRef<S>>::from_ref(state))
			}
		}
	};
}

pub(crate) use define_service;
