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

resource "aws_iam_policy" "dynamodb_stream_policy" {
  name        = "${var.environment}-dynamodb_stream_policy"
  description = "Grant access to dynamodb stream triggering lambdas."

  policy = <<EOF
{
    "Version": "2012-10-17",
    "Statement": [
        {
            "Effect": "Allow",
            "Action": "lambda:InvokeFunction",
            "Resource": "arn:aws:lambda:eu-west-2:676626137701:function:broadcastState*"
        },
        {
            "Effect": "Allow",
            "Action": [
                "logs:CreateLogGroup",
                "logs:CreateLogStream",
                "logs:PutLogEvents"
            ],
            "Resource": "arn:aws:logs:eu-west-2:676626137701:*"
        },
        {
            "Effect": "Allow",
            "Action": [
                "dynamodb:DescribeStream",
                "dynamodb:GetRecords",
                "dynamodb:GetShardIterator",
                "dynamodb:ListStreams"
            ],
            "Resource": "arn:aws:dynamodb:eu-west-2:676626137701:table/janky-werewolf-table/stream/*"
        },
        {
            "Effect": "Allow",
            "Action": [
                "sns:Publish"
            ],
            "Resource": [
                "*"
            ]
        },
        {
          "Action": [
            "execute-api:ManageConnections",
            "execute-api:Invoke"
          ],
          "Effect": "Allow",
          "Resource": "*"
        }
    ]
}
EOF

  tags = {
    Environment = var.environment
  }
}

resource "aws_iam_policy" "dynamodb_read_write_policy" {
  name        = "${var.environment}-dynamodb-read-write-access"
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
        "dynamodb:PutItem",
        "dynamodb:GetItem"
      ],
      "Effect": "Allow",
      "Resource": ["${aws_dynamodb_table.janky-werewolf-table.arn}",
                   "${aws_dynamodb_table.janky-werewolf-table.arn}/index/*"]
    },
    {
      "Action": [
        "execute-api:ManageConnections",
        "execute-api:Invoke"
      ],
      "Effect": "Allow",
      "Resource": "*"
    }
  ]
}
EOF

  tags = {
    Environment = var.environment
  }
}

resource "aws_iam_policy" "dynamodb_read_policy" {
  name        = "${var.environment}-dynamodb-read-only-access"
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

  tags = {
    Environment = var.environment
  }
}

resource "aws_iam_policy" "cloudwatch_log_policy" {
  name        = "${var.environment}-cloudwatch-write-log-policy"
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

  tags = {
    Environment = var.environment
  }
}

resource "aws_iam_role_policy_attachment" "cloudwatch_read_only_policy_attachment" {
  role       = aws_iam_role.iam_for_lambda_read_only.name
  policy_arn = aws_iam_policy.cloudwatch_log_policy.arn

  tags = {
    Environment = var.environment
  }
}

resource "aws_iam_role_policy_attachment" "stream_read_only_policy_attachment" {
  role       = aws_iam_role.iam_for_lambda_read_only.name
  policy_arn = aws_iam_policy.dynamodb_stream_policy.arn

  tags = {
    Environment = var.environment
  }
}

resource "aws_iam_role" "iam_for_lambda_read_only" {
  name               = "${var.environment}-iam_for_lambda_read_only"
  assume_role_policy = data.aws_iam_policy_document.policy.json

  tags = {
    Environment = var.environment
  }
}

resource "aws_iam_role_policy_attachment" "cloudwatch_policy_attachment" {
  role       = aws_iam_role.iam_for_lambda_read_write.name
  policy_arn = aws_iam_policy.cloudwatch_log_policy.arn

  tags = {
    Environment = var.environment
  }
}

resource "aws_iam_role_policy_attachment" "dynamodb_db_policy_attachment" {
  role       = aws_iam_role.iam_for_lambda_read_write.name
  policy_arn = aws_iam_policy.dynamodb_read_write_policy.arn

  tags = {
    Environment = var.environment
  }
}

resource "aws_iam_role" "iam_for_lambda_read_write" {
  name               = "${var.environment}-iam_for_lambda_read_write"
  assume_role_policy = data.aws_iam_policy_document.policy.json

  tags = {
    Environment = var.environment
  }
}
