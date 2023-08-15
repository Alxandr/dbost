output "database_endpoint" {
  type  = string
  value = aws_db_instance.dbost_db.endpoint
}
