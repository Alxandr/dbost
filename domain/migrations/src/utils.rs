use sea_orm_migration::{
	sea_orm::{ConnectionTrait, ExecResult},
	DbErr, SchemaManager,
};

pub(crate) async fn log_and_exec<'a, 'b>(
	manager: &'a SchemaManager<'b>,
	sql: impl AsRef<str>,
) -> Result<ExecResult, DbErr> {
	let sql = sql.as_ref();
	println!("Executing SQL: {}", sql);
	manager.get_connection().execute_unprepared(sql).await
}
