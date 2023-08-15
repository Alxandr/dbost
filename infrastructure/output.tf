output "database_address" {
  description = "The address of the database"
  value       = aws_db_instance.dbost_db.address
}

output "database_endpoint" {
  description = "The endpoint of the database"
  value       = aws_db_instance.dbost_db.endpoint
}

output "database_password" {
  description = "value of the database password"
  value       = random_password.db_master_password.result
  sensitive   = true
}
