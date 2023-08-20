##### ECS-Cluster #####
resource "aws_ecs_cluster" "cluster" {
  name = "dbost-cluster"
}

##### dBost task (service) definition #####
data "aws_ecs_task_definition" "dbost" {
  task_definition = "dbost"
}

data "aws_ecs_task_definition" "dbost_db_cleaner" {
  task_definition = "dbost-db-cleaner"
}

##### AWS ECS-SERVICE #####
resource "aws_ecs_service" "dbost" {
  cluster         = aws_ecs_cluster.cluster.id             # ECS Cluster ID
  desired_count   = 1                                      # Number of tasks running
  launch_type     = "FARGATE"                              # Cluster type [ECS OR FARGATE]
  name            = "dbost"                                # Name of service
  task_definition = data.aws_ecs_task_definition.dbost.arn # Attach the task to service

  network_configuration {
    subnets          = module.vpc.public_subnets
    security_groups  = [aws_security_group.public.id]
    assign_public_ip = true
  }

  load_balancer {
    container_name   = "dbost"
    container_port   = "80"
    target_group_arn = aws_alb_target_group.alb_public_webservice_target_group.arn
  }

  depends_on = [aws_lb_listener.lb_listener-webservice-https]

  lifecycle {
    ignore_changes = [task_definition]
  }
}

##### CLOUDWATCH SCHEDULE #####
resource "aws_scheduler_schedule" "dbost_db_clean_schedule" {
  name                = "dbost-db-clean-schedule"
  schedule_expression = "rate(15 minutes)"
  description         = "Cleans dbost database every 6 hours"
  # role_arn            = var.event_rule_role_arn
  # is_enabled = true

  flexible_time_window {
    mode                      = "FLEXIBLE"
    maximum_window_in_minutes = 30
  }

  target {
    arn      = aws_ecs_cluster.cluster.arn
    role_arn = aws_iam_role.ecs_agent.arn

    ecs_parameters {
      task_count          = 1
      task_definition_arn = data.aws_ecs_task_definition.dbost_db_cleaner.arn_without_revision
      launch_type         = "FARGATE"

      network_configuration {
        subnets          = module.vpc.public_subnets
        security_groups  = [aws_security_group.public.id]
        assign_public_ip = true
      }
    }

    retry_policy {
      maximum_retry_attempts = 2
    }

    dead_letter_config {
      arn = aws_sqs_queue.dbost_db_schedule_dlq.arn
    }
  }
}

resource "aws_sqs_queue" "dbost_db_schedule_dlq" {
  name                      = "dbost-db-schedule-dlq"
  message_retention_seconds = 60 * 60 * 24 * 7 # 7 days
}
