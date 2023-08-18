pub use sea_orm_migration::prelude::*;
pub mod prelude;

mod funcs;
mod tables;
mod utils;

mod m20220101_000001_create_table;
mod m20230730_233755_create_theme_song_entity;
mod m20230801_120415_gen_random_uuid;
mod m20230802_075725_search_indices;
mod m20230807_142613_versioning;
mod m20230811_170956_sessions;
mod m20230813_063316_artwork;
mod m20230813_131452_hot_indices;
mod m20230818_124952_descriptions;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
	fn migrations() -> Vec<Box<dyn MigrationTrait>> {
		vec![
			Box::new(m20220101_000001_create_table::Migration),
			Box::new(m20230730_233755_create_theme_song_entity::Migration),
			Box::new(m20230801_120415_gen_random_uuid::Migration),
			Box::new(m20230802_075725_search_indices::Migration),
			Box::new(m20230807_142613_versioning::Migration),
			Box::new(m20230811_170956_sessions::Migration),
			Box::new(m20230813_063316_artwork::Migration),
			Box::new(m20230813_131452_hot_indices::Migration),
			Box::new(m20230818_124952_descriptions::Migration),
		]
	}
}
