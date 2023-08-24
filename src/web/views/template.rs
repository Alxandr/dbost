use dbost_entities::user;
use dbost_session::Session;
use rstml_component::{write_html, HtmlComponent, HtmlContent, HtmlFormatter};
use std::{borrow::Cow, fmt};

use crate::assets::BuiltAssets;

#[derive(HtmlComponent)]
struct NavSearchBox;

impl HtmlContent for NavSearchBox {
	fn fmt(self, formatter: &mut HtmlFormatter) -> fmt::Result {
		write_html!(formatter,
			<div class="form-control">
				<input type="text" placeholder="Search" class="w-24 input input-bordered sm:w-auto" />
			</div>
		)
	}
}

#[derive(HtmlComponent)]
struct UserDropdown<'a> {
	user: Option<&'a user::Model>,
}

impl<'a> HtmlContent for UserDropdown<'a> {
	fn fmt(self, formatter: &mut HtmlFormatter) -> fmt::Result {
		match self.user {
			None => write_html!(formatter,
				<div>
					<label tabindex="0" class="btn btn-ghost btn-circle avatar">
						<a href="/auth/login/github" class="w-10 rounded-full" hx-boost="false">
							<svg width="20" height="20" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512" class="inline-block w-5 h-5 fill-current md:h-6 md:w-6">
								<path d="M256,32C132.3,32,32,134.9,32,261.7c0,101.5,64.2,187.5,153.2,217.9a17.56,17.56,0,0,0,3.8.4c8.3,0,11.5-6.1,11.5-11.4,0-5.5-.2-19.9-.3-39.1a102.4,102.4,0,0,1-22.6,2.7c-43.1,0-52.9-33.5-52.9-33.5-10.2-26.5-24.9-33.6-24.9-33.6-19.5-13.7-.1-14.1,1.4-14.1h.1c22.5,2,34.3,23.8,34.3,23.8,11.2,19.6,26.2,25.1,39.6,25.1a63,63,0,0,0,25.6-6c2-14.8,7.8-24.9,14.2-30.7-49.7-5.8-102-25.5-102-113.5,0-25.1,8.7-45.6,23-61.6-2.3-5.8-10-29.2,2.2-60.8a18.64,18.64,0,0,1,5-.5c8.1,0,26.4,3.1,56.6,24.1a208.21,208.21,0,0,1,112.2,0c30.2-21,48.5-24.1,56.6-24.1a18.64,18.64,0,0,1,5,.5c12.2,31.6,4.5,55,2.2,60.8,14.3,16.1,23,36.6,23,61.6,0,88.2-52.4,107.6-102.3,113.3,8,7.1,15.2,21.1,15.2,42.5,0,30.7-.3,55.5-.3,63,0,5.4,3.1,11.5,11.4,11.5a19.35,19.35,0,0,0,4-.4C415.9,449.2,480,363.1,480,261.7,480,134.9,379.7,32,256,32Z" />
							</svg>
						</a>
					</label>
				</div>
			),
			Some(user) => {
				let avatar_url = user
					.avatar_url
					.as_deref()
					.map(Cow::Borrowed)
					.unwrap_or_else(|| {
						Cow::Owned(format!(
							"https://www.gravatar.com/avatar/{:x}?d=mp",
							md5::compute(user.email.as_bytes())
						))
					});
				write_html!(formatter,
					<div class="dropdown dropdown-end" id="navbar-user">
						<label tabindex="0" class="btn btn-ghost btn-circle avatar">
							<div class="w-10 rounded-full">
								<img src=&*avatar_url referrerpolicy="no-referrer" />
							</div>
						</label>

						<ul tabindex="0" class="mt-3 z-[1] p-2 shadow menu menu-sm dropdown-content bg-base-100 rounded-box w-52">
							<li><a>"Profile"</a></li>
							<li><a href="/auth/logout">"Logout"</a></li>
						</ul>
					</div>
				)
			}
		}
	}
}

#[derive(HtmlComponent)]
struct NavBar<'a> {
	user: Option<&'a user::Model>,
}

impl<'a> HtmlContent for NavBar<'a> {
	fn fmt(self, formatter: &mut HtmlFormatter) -> fmt::Result {
		write_html!(formatter,
			<nav class="navbar bg-base-100">
				// <div class="flex-none">
				// 	<button class="btn btn-square btn-ghost">
				// 		<svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" class="inline-block w-5 h-5 stroke-current">
				// 			<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 6h16M4 12h16M4 18h16" />
				// 		</svg>
				// 	</button>
				// </div>

				<div class="flex-1">
					<a class="text-xl normal-case btn btn-ghost hover:bg-transparent" href="/">"dBost"</a>
					<span class="normal-case text-normal">"| ˈdi: buːst |"</span>
				</div>

				// search
				<div class="flex-none gap-2">
					<NavSearchBox />
					<UserDropdown user=self.user />
				</div>
			</nav>
		)
	}
}

#[derive(HtmlComponent)]
pub struct Template<'a, T, C>
where
	T: AsRef<str>,
	C: HtmlContent,
{
	pub title: T,
	pub children: C,
	pub session: &'a Session,
}

impl<'a, T, C> HtmlContent for Template<'a, T, C>
where
	T: AsRef<str>,
	C: HtmlContent,
{
	fn fmt(self, formatter: &mut HtmlFormatter) -> fmt::Result {
		let assets = BuiltAssets::assets();

		write_html!(formatter,
			<!DOCTYPE html>
			<html hx-boost="true">
				<head>
					<meta charset="UTF-8" />
					<link rel="apple-touch-icon" sizes="57x57" href="/public/icon/apple-icon-57x57.png">
					<link rel="apple-touch-icon" sizes="60x60" href="/public/icon/apple-icon-60x60.png">
					<link rel="apple-touch-icon" sizes="72x72" href="/public/icon/apple-icon-72x72.png">
					<link rel="apple-touch-icon" sizes="76x76" href="/public/icon/apple-icon-76x76.png">
					<link rel="apple-touch-icon" sizes="114x114" href="/public/icon/apple-icon-114x114.png">
					<link rel="apple-touch-icon" sizes="120x120" href="/public/icon/apple-icon-120x120.png">
					<link rel="apple-touch-icon" sizes="144x144" href="/public/icon/apple-icon-144x144.png">
					<link rel="apple-touch-icon" sizes="152x152" href="/public/icon/apple-icon-152x152.png">
					<link rel="apple-touch-icon" sizes="180x180" href="/public/icon/apple-icon-180x180.png">
					<link rel="icon" type="image/png" sizes="192x192"  href="/public/icon/android-icon-192x192.png">
					<link rel="icon" type="image/png" sizes="32x32" href="/public/icon/favicon-32x32.png">
					<link rel="icon" type="image/png" sizes="96x96" href="/public/icon/favicon-96x96.png">
					<link rel="icon" type="image/png" sizes="16x16" href="/public/icon/favicon-16x16.png">
					<link rel="icon" type="image/x-icon" href="/public/icon/favicon.ico">
					<link rel="manifest" href="/public/dbost.webmanifest">
					<meta name="msapplication-TileColor" content="#1d232a">
					<meta name="msapplication-TileImage" content="/public/icon/ms-icon-144x144.png">
					<meta name="theme-color" content="#1d232a">
					<meta name="viewport" content="width=device-width, initial-scale=1" />
					<title>{self.title.as_ref()}" | dBost"</title>
					<link rel="stylesheet" type="text/css" href=("/public/", assets.css) />
					<script src=("/public/", assets.js) type="module" />
				</head>
				<body>
					<NavBar user=self.session.user().as_deref() />
					<main class="p-8">
						{self.children}
					</main>
				</body>
			</html>
		)
	}
}
