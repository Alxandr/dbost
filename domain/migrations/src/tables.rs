use sea_orm_migration::prelude::*;

#[derive(Iden, Clone, Copy)]
pub enum Series {
	Table,
	Id,
	Name,
	#[iden = "tvdb_id"]
	TvDbId,
	ThemeSongId,
}

#[derive(Iden, Clone, Copy)]
pub enum Season {
	Table,
	Id,
	Name,
	SeriesId,
	Number,
	#[iden = "tvdb_id"]
	TvDbId,
	ThemeSongId,
}

#[derive(Iden, Clone, Copy)]
pub enum ThemeSong {
	Table,
	Id,
	Name,
	#[iden = "youtube_id"]
	YouTubeId,
	#[iden = "youtube_starts_at"]
	YouTubeStartsAt,
	#[iden = "youtube_ends_at"]
	YouTubeEndsAt,
}

#[derive(Iden, Clone, Copy)]
pub enum Versioned {
	#[iden = "_version"]
	Version,
}

#[derive(Iden, Clone, Copy)]
pub enum Session {
	Table,
	Id,
	#[iden = "ctime"]
	CreateTime,
	#[iden = "atime"]
	AccessTime,
	#[iden = "etime"]
	ExpiryTime,
	UserId,
}

#[derive(Iden, Clone, Copy)]
pub enum User {
	Table,
	Id,
	DisplayName,
	Email,
}

#[derive(Iden, Clone, Copy)]
pub enum UserLink {
	Table,
	Service,
	UserId,
	#[iden = "service_userid"]
	ServiceUserId,
}
