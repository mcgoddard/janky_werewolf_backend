module "api_lambda" {
  source             = "../modules/lambda"

  lambda_name        = "api_lambda"
  iam_role_arn       = aws_iam_role.iam_for_lambda_read_write.arn
  bin_dir            = var.bin_dir
  aws_region         = var.aws_region
  aws_account_id     = var.aws_account_id
  environment        = var.environment
  table_name         = aws_dynamodb_table.janky-werewolf-table.name
}

module "broadcast_lambda" {
  source             = "../modules/lambda"

  lambda_name        = "broadcast_lambda"
  iam_role_arn       = aws_iam_role.iam_for_lambda_read_only.arn
  bin_dir            = var.bin_dir
  aws_region         = var.aws_region
  aws_account_id     = var.aws_account_id
  api_gateway_url    = aws_apigatewayv2_stage.stage.invoke_url
  environment        = var.environment
  table_name         = aws_dynamodb_table.janky-werewolf-table.name
}
