import { Construct } from 'constructs';
import * as cdk from 'aws-cdk-lib/core';
import * as lambda from 'aws-cdk-lib/aws-lambda';
import * as ec2 from 'aws-cdk-lib/aws-ec2';
import { HttpApi, HttpMethod } from 'aws-cdk-lib/aws-apigatewayv2';
import path from 'path';
import { HttpLambdaIntegration } from 'aws-cdk-lib/aws-apigatewayv2-integrations';

export class HistoricalSeptaStack extends cdk.Stack {
  constructor(scope: Construct, id: string, props?: cdk.StackProps) {
    super(scope, id, props);
    const rds_privateVpc = ec2.Vpc.fromLookup(this, 'Api-RdsVpc', {
      vpcId: 'vpc-099906c477bf80070'
    });

    const lambda_Get = new lambda.Function(this, 'historical_septa-Get', {
      vpc: rds_privateVpc,
      runtime: lambda.Runtime.PROVIDED_AL2023,
      code: lambda.Code.fromAsset(path.join(process.cwd(), '../target/lambda/lambda-get/bootstrap.zip')),
      handler: 'bootstrap',
      memorySize: 128,
      timeout: cdk.Duration.minutes(2),
      architecture: lambda.Architecture.X86_64,
      environment: {
        "DATABASE_HOST": process.env["DATABASE_HOST"]!,
        "DATABASE_NAME": process.env["DATABASE_NAME"]!,
        "DATABASE_PORT": process.env["DATABASE_PORT"]!,
        "DATABASE_USER": process.env["DATABASE_USER"]!,
        "DATABASE_PASS": process.env["DATABASE_PASS"]!,
        "RUST_LOG": "DEBUG",
      }
    })

    const lambda_Get_Integration = new HttpLambdaIntegration('lambda_get-integration', lambda_Get)

    const historical_septa_Api = new HttpApi(this, 'historical_septa-RestApi');
    historical_septa_Api.addRoutes({
      path: '/current',
      methods: [HttpMethod.GET],
      integration: lambda_Get_Integration,
    })

    new cdk.CfnOutput(this, 'historical_septa_ApiUrl', {
      description: 'The URL of the Historical Septa API Gateway',
      value: `https://${historical_septa_Api.apiId}.execute-api.${this.region}.amazonaws.com`
    })

  }
}
