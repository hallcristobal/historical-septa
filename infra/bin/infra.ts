#!/usr/bin/env node
import * as cdk from "aws-cdk-lib/core";
import { HistoricalSeptaStack } from "../lib/infra-stack";
import * as dotenv from "dotenv";
import path from "path";

const parentEnvPath = path.join(process.cwd(), "../.env");
dotenv.config({ path: parentEnvPath });

const app = new cdk.App();
const REGION = process.env.AWS_REGION;
if (!REGION || REGION.length < 3) {
  console.error(`Invalid region: ${REGION}`);
  process.exit(1);
}

new HistoricalSeptaStack(app, "HistoricalSeptaStack", {
  /* If you don't specify 'env', this stack will be environment-agnostic.
   * Account/Region-dependent features and context lookups will not work,
   * but a single synthesized template can be deployed anywhere. */

  /* Uncomment the next line to specialize this stack for the AWS Account
   * and Region that are implied by the current CLI configuration. */
  env: { account: '740344857772', region: REGION },

  /* Uncomment the next line if you know exactly what Account and Region you
   * want to deploy the stack to. */
  // env: { account: '123456789012', region: 'us-east-1' },

  /* For more information, see https://docs.aws.amazon.com/cdk/latest/guide/environments.html */
});
