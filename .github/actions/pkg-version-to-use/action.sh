#!/usr/bin/env bash

set -eu -o pipefail

LATEST_VERSION=$(jq -r .latest_version "$PKG_FILE")

if [ -z "$PKG_VERSION" ]; then
  PKG_VERSION="$LATEST_VERSION"
fi

if jq -r '.versions | keys[]' "$PKG_FILE" | grep -e "$PKG_VERSION"; then
  (
    echo "version=$PKG_VERSION"
    if [ "$LATEST_VERSION" == "$PKG_VERSION" ]; then
      echo is_latest=true
    else
      echo is_latest=false
    fi
  ) | tee -a $GITHUB_OUTPUT
else
  echo "The specified version ($PKG_VERSION) is not listed in the package file ($PKG_FILE)" >&2
  exit 1
fi
