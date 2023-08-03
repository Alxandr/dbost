pub use sea_orm_migration::prelude::*;

mod m20220101_000001_create_table;
mod m20230730_233755_create_theme_song_entity;
mod m20230801_120415_gen_random_uuid;
mod m20230802_075725_search_indices;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
	fn migrations() -> Vec<Box<dyn MigrationTrait>> {
		vec![
            Box::new(m20220101_000001_create_table::Migration),
            Box::new(m20230730_233755_create_theme_song_entity::Migration),
            Box::new(m20230801_120415_gen_random_uuid::Migration),
            Box::new(m20230802_075725_search_indices::Migration),
        ]
	}
}
