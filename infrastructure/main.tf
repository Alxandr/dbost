resource "aws_secretsmanager_secret" "web" {
  name = "dbost_web"
}

resource "aws_secretsmanager_secret" "tvdb" {
  name = "dbost_tvdb"
}
