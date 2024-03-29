resource "aws_apigatewayv2_api" "api" {
  name                       = "${var.environment}-api"
  protocol_type              = "WEBSOCKET"
  route_selection_expression = "$request.body.action"

  tags = {
    Environment = var.environment
  }
}

resource "aws_apigatewayv2_route" "default_route" {
  api_id    = aws_apigatewayv2_api.api.id
  route_key = "$default"
  target    = "integrations/${aws_apigatewayv2_integration.default_route_integration.id}"
}

resource "aws_apigatewayv2_integration" "default_route_integration" {
  api_id           = aws_apigatewayv2_api.api.id
  integration_type = "AWS_PROXY"

  connection_type           = "INTERNET"
  content_handling_strategy = "CONVERT_TO_TEXT"
  description               = "Integration for the default API route"
  integration_uri           = module.api_lambda.invoke_arn
  passthrough_behavior      = "WHEN_NO_MATCH"
}

resource "aws_apigatewayv2_stage" "stage" {
  api_id = aws_apigatewayv2_api.api.id
  name   = "dev"
  deployment_id = aws_apigatewayv2_deployment.deployment.id

  tags = {
    Environment = var.environment
  }
}

resource "aws_apigatewayv2_deployment" "deployment" {
  api_id      = aws_apigatewayv2_api.api.id
  description = "API deployment"

  lifecycle {
    create_before_destroy = true
  }

  triggers = {
    redeployment = sha1(join(",", list(
      md5(file("api-gateway.tf"))
    )))
  }

  depends_on = [
    aws_apigatewayv2_route.default_route,
    aws_apigatewayv2_integration.default_route_integration,
  ]
}
