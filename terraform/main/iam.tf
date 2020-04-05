data "aws_iam_policy_document" "policy" {
  statement {
    sid    = ""
    effect = "Allow"

    principals {
      identifiers = ["lambda.amazonaws.com"]
      type        = "Service"
    }

    actions = ["sts:AssumeRole"]
  }
}

resource "aws_iam_policy" "dynamodb_read_write_policy" {
  name        = "dynamodb-read-write-access"
  description = "Grant access to the dynamodb table."

  policy = <<EOF
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Action": [
        "dynamodb:Query",
        "dynamodb:Delete*",
        "dynamodb:Update*",
        "dynamodb:PutItem"
      ],
      "Effect": "Allow",
      "Resource": ["${aws_dynamodb_table.janky-werewolf-table.arn}",
                   "${aws_dynamodb_table.janky-werewolf-table.arn}/index/*"]
    }
  ]
}
EOF
}

resource "aws_iam_policy" "dynamodb_read_policy" {
  name        = "dynamodb-read-only-access"
  description = "Grant access read only to the dynamodb table."

  policy = <<EOF
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Action": [
        "dynamodb:Query"
      ],
      "Effect": "Allow",
      "Resource": ["${aws_dynamodb_table.janky-werewolf-table.arn}",
                   "${aws_dynamodb_table.janky-werewolf-table.arn}/index/*"]
    }
  ]
}
EOF
}

resource "aws_iam_policy" "cloudwatch_log_policy" {
  name        = "cloudwatch-write-log-policy"
  description = "Grant access to write cloudwatch logs."

  policy = <<EOF
{
  "Version": "2012-10-17",
  "Statement": [
    {
            "Effect": "Allow",
            "Action": [
                "logs:CreateLogGroup",
                "logs:CreateLogStream",
                "logs:PutLogEvents"
            ],
            "Resource": "*"
        }
  ]
}
EOF
}

resource "aws_iam_role_policy_attachment" "cloudwatch_read_only_policy_attachment" {
  role       = aws_iam_role.iam_for_lambda_read_only.name
  policy_arn = aws_iam_policy.cloudwatch_log_policy.arn
}

resource "aws_iam_role_policy_attachment" "cloudwatch_policy_attachment" {
  role       = aws_iam_role.iam_for_lambda_read_write.name
  policy_arn = aws_iam_policy.cloudwatch_log_policy.arn
}

resource "aws_iam_role_policy_attachment" "dynamodb_db_read_only_policy_attachment" {
  role       = aws_iam_role.iam_for_lambda_read_only.name
  policy_arn = aws_iam_policy.dynamodb_read_policy.arn
}

resource "aws_iam_role" "iam_for_lambda_read_only" {
  name               = "iam_for_lambda_read_only"
  assume_role_policy = data.aws_iam_policy_document.policy.json
}

resource "aws_iam_role_policy_attachment" "dynamodb_db_policy_attachment" {
  role       = aws_iam_role.iam_for_lambda_read_write.name
  policy_arn = aws_iam_policy.dynamodb_read_write_policy.arn
}

resource "aws_iam_role" "iam_for_lambda_read_write" {
  name               = "iam_for_lambda_read_write"
  assume_role_policy = data.aws_iam_policy_document.policy.json
}
