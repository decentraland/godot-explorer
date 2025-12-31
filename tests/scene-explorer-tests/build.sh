#!/bin/bash

echo "Installing and building all folders"

rm -rf node_modules

JS_RUNTIME_PACKAGE="https://sdk-team-cdn.decentraland.org/@dcl/js-sdk-toolchain/branch/psquad/test-framework-tool/@dcl/js-runtime/dcl-js-runtime-7.3.36-7291127796.commit-b299b0d.tgz"
SDK_PACKAGE="https://sdk-team-cdn.decentraland.org/@dcl/js-sdk-toolchain/branch/psquad/test-framework-tool/dcl-sdk-7.3.36-7291127796.commit-b299b0d.tgz"

npm i $SDK_PACKAGE $JS_RUNTIME_PACKAGE --legacy-peer-deps

# then the rest of the dependencies
npm install --legacy-peer-deps

npm run sync

npm ls @dcl/sdk

# clean the git state of package.json(s)
git add */package.json package.json

# and fail if git state is dirty
git diff --ignore-cr-at-eol --exit-code .

if [[ $? -eq 1 ]]; then
  echo "GIT IS ON DIRTY STATE 🔴 Please run 'npm run update-parcels' locally and commit"
  exit 1
fi

# ensure all packages are on sync
npm run sync && npm run test-sync

# and lastly build scenes
npm run build