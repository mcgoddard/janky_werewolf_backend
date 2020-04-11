data "archive_file" "zip" {
  type        = "zip"
  output_path = "${var.bin_dir}/${var.lambda_name}/release.zip"
  source_dir  = "${var.bin_dir}/${var.lambda_name}/release"
}

resource "aws_lambda_function" "lambda" {
  function_name = var.lambda_name

  filename         = data.archive_file.zip.output_path
  source_code_hash = data.archive_file.zip.output_base64sha256

  role    = var.iam_role_arn
  handler = "${var.lambda_name}.lambda_handler"
  runtime = "provided"

  environment {
    variables = {
      tableName  = "janky-werewolf-table"
      domainName = var.api_gateway_domain
      stage      = var.api_gateway_stage
    }
  }
}

resource "aws_lambda_permission" "apigw_lambda" {
  statement_id  = "AllowExecutionFromAPIGateway"
  action        = "lambda:InvokeFunction"
  function_name = aws_lambda_function.lambda.function_name
  principal     = "apigateway.amazonaws.com"

  # More: http://docs.aws.amazon.com/apigateway/latest/developerguide/api-gateway-control-access-using-iam-policies-to-invoke-api.html
  source_arn = "arn:aws:execute-api:${var.aws_region}:${var.aws_account_id}:*"
}
