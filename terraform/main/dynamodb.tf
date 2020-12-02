

resource "aws_dynamodb_table" "janky-werewolf-table" {
  name           = "${var.environment}-janky-werewolf-table"
  billing_mode   = "PAY_PER_REQUEST"
  hash_key       = "lobby_id"

  attribute {
    name = "lobby_id"
    type = "S"
  }

  ttl {
    attribute_name = "ttl"
    enabled        = true
  }

  stream_enabled   = true
  stream_view_type = "NEW_IMAGE"

  tags = {
    Environment = var.environment
  }
}

resource "aws_lambda_event_source_mapping" "broadcast-state-mapping" {
  event_source_arn       = aws_dynamodb_table.janky-werewolf-table.stream_arn
  function_name          = module.broadcast_lambda.lambda_arn
  starting_position      = "LATEST"
  batch_size             = 1
  maximum_retry_attempts = 2
}
