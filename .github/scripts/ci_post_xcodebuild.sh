#!/bin/bash
set -e

# This script runs in Xcode Cloud after the build completes
# It uploads dSYMs to Sentry for crash symbolication

if [[ "$CI_XCODEBUILD_ACTION" != "archive" ]]; then
  echo "Skipping dSYM upload - not an archive action"
  exit 0
fi

echo "Uploading dSYMs to Sentry..."

# Check required environment variables (set in Xcode Cloud workflow)
if [[ -z "$SENTRY_AUTH_TOKEN" || -z "$SENTRY_ORG" || -z "$SENTRY_PROJECT" ]]; then
  echo "Warning: Sentry environment variables not configured"
  echo "Set SENTRY_AUTH_TOKEN, SENTRY_ORG, and SENTRY_PROJECT in Xcode Cloud"
  exit 0
fi

# Install sentry-cli
curl -sL https://sentry.io/get-cli/ | bash

# Find the archive path
DSYM_PATH="$CI_ARCHIVE_PATH/dSYMs"

echo "Looking for dSYMs in: $DSYM_PATH"
ls -la "$DSYM_PATH" 2>/dev/null || echo "dSYMs directory not found"

if [[ ! -d "$DSYM_PATH" ]]; then
  echo "Warning: dSYMs directory not found at $DSYM_PATH"
  exit 0
fi

# Upload to Sentry
sentry-cli upload-dif \
  --org "$SENTRY_ORG" \
  --project "$SENTRY_PROJECT" \
  --auth-token "$SENTRY_AUTH_TOKEN" \
  --include-sources \
  --wait \
  "$DSYM_PATH"

echo "dSYMs uploaded successfully"
