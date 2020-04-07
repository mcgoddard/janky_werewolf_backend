output "signup_arn" {
  value = module.connect_lambda.lambda_arn
}

output "stream_arn" {
  value = aws_dynamodb_table.janky-werewolf-table.stream_arn
}
