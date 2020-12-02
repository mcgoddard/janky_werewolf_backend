output "api_arn" {
  value = module.api_lambda.lambda_arn
}

output "broadcast_arn" {
  value = module.broadcast_lambda.lambda_arn
}

output "stream_arn" {
  value = aws_dynamodb_table.janky-werewolf-table.stream_arn
}
