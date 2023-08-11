use macros::define_service;
use sea_orm::DatabaseConnection;

mod macros;

define_service! {
	pub struct SeriesService {
		db: DatabaseConnection,
	}
}

// define_service! {
// 	pub struct DBostService {
// 		db: DatabaseConnection,
// 		series: SeriesService,
// 	}
// }
