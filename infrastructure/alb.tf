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
  target_type      = "ip"
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

  depends_on = [aws_acm_certificate_validation.cert_validation]
}

# SSL Certificate
resource "aws_acm_certificate" "ssl_certificate" {
  domain_name               = var.domain_name
  subject_alternative_names = ["*.${var.domain_name}"]
  validation_method         = "DNS"
  key_algorithm             = "EC_secp521r1"

  lifecycle {
    create_before_destroy = true
  }
}
# SSL Certificate validation
resource "aws_acm_certificate_validation" "cert_validation" {
  certificate_arn = aws_acm_certificate.ssl_certificate.arn
}

### R53 Zone ###
resource "aws_route53_zone" "dbost" {
  name = var.domain_name
}


### R53 Records ###
resource "aws_route53_record" "www" {
  zone_id = aws_route53_zone.dbost.zone_id
  name    = var.domain_name
  type    = "A"

  alias {
    name                   = aws_lb.loadbalancer.dns_name
    zone_id                = aws_lb.loadbalancer.zone_id
    evaluate_target_health = true
  }
}

resource "aws_route53_record" "cert_dns_validation" {
  for_each = {
    for dvo in aws_acm_certificate.ssl_certificate.domain_validation_options : dvo.domain_name => {
      name   = dvo.resource_record_name
      record = dvo.resource_record_value
      type   = dvo.resource_record_type
    }
  }

  allow_overwrite = true
  name            = each.value.name
  records         = [each.value.record]
  ttl             = 60
  type            = each.value.type
  zone_id         = aws_route53_zone.dbost.zone_id
}

### DNSIMPLE domain delegation ###
# Create a domain delegation
resource "dnsimple_domain_delegation" "dbost" {
  domain       = var.domain_name
  name_servers = sort(aws_route53_zone.dbost.name_servers)

  lifecycle {
    ignore_changes = [name_servers]
  }
}
