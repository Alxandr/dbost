##### ALB - Application Load Balancing #####
##### ALB - Load Balancer #####
resource "aws_lb" "loadbalancer" {
  name                 = "dbost-lb"
  internal             = "false" # internal = true else false
  load_balancer_type   = "application"
  preserve_host_header = true
  subnets              = module.vpc.public_subnets # Subnets públicas
  security_groups      = [aws_security_group.public.id]
}

##### ALB - Target Groups #####


resource "aws_alb_target_group" "alb_public_webservice_target_group" {
  name             = "dbost-public-webservice-tg"
  port             = "80"
  protocol         = "HTTP"
  protocol_version = "HTTP2"
  vpc_id           = module.vpc.vpc_id

  health_check {
    healthy_threshold   = "3"
    interval            = "15"
    path                = "/healthz"
    protocol            = "HTTP"
    unhealthy_threshold = "10"
    timeout             = "10"
  }
}
##### ALB - Listeners #####

resource "aws_lb_listener" "lb_listener-webservice-https-redirect" {
  load_balancer_arn = aws_lb.loadbalancer.arn
  port              = "80"
  protocol          = "HTTP"
  default_action {
    type = "redirect"
    redirect {
      port        = "443"
      protocol    = "HTTPS"
      status_code = "HTTP_301"
    }
  }
}

resource "aws_lb_listener" "lb_listener-webservice-https" {
  load_balancer_arn = aws_lb.loadbalancer.arn
  port              = "443"
  protocol          = "HTTPS"
  ssl_policy        = "ELBSecurityPolicy-2016-08"
  certificate_arn   = aws_acm_certificate.ssl_certificate.arn

  default_action {
    type             = "forward"
    target_group_arn = aws_alb_target_group.alb_public_webservice_target_group.id
  }
}
