output "database_address" {
  description = "The address of the database"
  value       = aws_db_instance.dbost_db.address
}

output "database_endpoint" {
  description = "The endpoint of the database"
  value       = aws_db_instance.dbost_db.endpoint
}
