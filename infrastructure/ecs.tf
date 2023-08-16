##### ECS-Cluster #####
resource "aws_ecs_cluster" "cluster" {
  name = "dbost-cluster"
}

##### DB migrator task #####
resource "aws_ecs_task_definition" "dbost" {
  family                   = "dbost"
  requires_compatibilities = ["FARGATE"]
  network_mode             = "awsvpc"
  cpu                      = 512
  memory                   = 1024
  container_definitions = jsonencode([
    {
      name                   = "dbost-db-migrator"
      image                  = "ghcr.io/alxandr/dbost/migrator:latest"
      essential              = false
      readonlyRootFilesystem = true
      memory                 = 1024
      environment = [
        {
          name  = "DATABASE_URL"
          value = "postgres://${postgresql_role.migrator.name}:${postgresql_role.migrator.password}@${aws_db_instance.dbost_db.endpoint}/postgres"
        },
        {
          name  = "DATABASE_SCHEMA"
          value = "public"
        }
      ]
    },
    {
      name                   = "dbost"
      image                  = "ghcr.io/alxandr/dbost:latest"
      readonlyRootFilesystem = true
      memory                 = 1024
      dependsOn = [
        {
          containerName = "dbost-db-migrator"
          condition     = "SUCCESS"
        }
      ]
      environment = [
        {
          name  = "DATABASE_URL"
          value = "postgres://${postgresql_role.migrator.name}:${postgresql_role.migrator.password}@${aws_db_instance.dbost_db.endpoint}/postgres"
        },
        {
          name  = "DATABASE_SCHEMA"
          value = "public"
        }
      ]
    }
  ])
}
