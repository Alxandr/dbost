##### ECS-Cluster #####
resource "aws_ecs_cluster" "cluster" {
  name = "dbost-cluster"
}

##### dBost task (service) definition #####
data "aws_ecs_task_definition" "dbost" {
  family = "dbost"
}

data "aws_ecs_task_definition" "dbost_db_cleaner" {
  family = "dbost-db-cleaner"
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
