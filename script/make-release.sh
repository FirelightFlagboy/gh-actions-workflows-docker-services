set -eu -o pipefail

OS=linux
ARCH=amd64

# Skip signed git commit and push.
SKIP_SIGN=${SKIP_SIGN:-0}
SKIP_ARTIFACT_UPLOAD=${SKIP_ARTIFACT_UPLOAD:-0}
BUILD_IN_DOCKER=${BUILD_IN_DOCKER:-0}

if [ $SKIP_SIGN -eq 1 ]; then
  COMMIT_SIGN_ARGS="--no-gpg-sign"
  TAG_SIGN_ARGS="--no-sign"
else
  COMMIT_SIGN_ARGS="--gpg-sign"
  TAG_SIGN_ARGS="--sign"
fi

SCRIPTDIR=${SCRIPTDIR:=$(dirname $(realpath -s "$0"))}
ROOTDIR=${ROOTDIR:=$(realpath -s "$SCRIPTDIR/..")}
SKIP_RELEASE_CREATION=${SKIP_RELEASE_CREATION:-0}

NEW_VERSION=${1:?Missing new version}
RELEASE_DATE=$(date --rfc-3339=date --utc)

TEMP_FILES=()

function cleanup_temp_files {
  for temp_file in "${TEMP_FILES[@]}" ; do
    rm $temp_file
  done
}

trap "cleanup_temp_files" EXIT INT

function build_new_version_for_bin {
  echo "Building bin for $NEW_VERSION"
  sed -i "s/^version = \".*\"$/version = \"$NEW_VERSION\"/" Cargo.toml

  cargo check # Update lock with updated version

  if [ $BUILD_IN_DOCKER -eq 1 ]; then
    DOCKER_WORKING_DIR=/tmp

    RUST_VERSION=$(sed -n 's/^channel = "\(.*\)"$/\1/p' rust-toolchain.toml)
    sed -i "s/^FROM rust:.*$/FROM rust:$RUST_VERSION-slim/" $SCRIPTDIR/build.dockerfile

    # Build docker image
    docker build -t rust-builder -f $SCRIPTDIR/build.dockerfile .

    # Build in docker
    docker run --rm --name rust-builder \
      --mount type=bind,source=$ROOTDIR/target/release/pkg-info-updater,target=/tmp/result/pkg-info-updater \
      rust-builder \
      cp -v /tmp/build/target/release/pkg-info-updater /tmp/result/pkg-info-updater
      # cargo build --release
  else
    cargo build --release
  fi
}

function add_new_version_to_pkg_file {
  local FILENAME=pkg-info-updater-$OS-$ARCH
  local DOWNLOAD_URL=https://github.com/FirelightFlagboy/gh-actions-workflows-docker-services/releases/download/v$NEW_VERSION/pkg-info-updater-linux-amd64
  local SHA512SUM=$(sha512sum target/release/pkg-info-updater | cut -f 1 -d' ')
  local TEMP_FILE=$(mktemp)
  TEMP_FILES+=($TEMP_FILE)

  echo "Adding new version to pkg-file"
  jq \
    ".versions[\"$NEW_VERSION\"] = {\"$ARCH\":{\"filename\":\"$FILENAME\",\"download_url\":\"$DOWNLOAD_URL\",\"digest\":\"sha512:$SHA512SUM\"}} | .latest_version = \"$NEW_VERSION\"" \
    pkg-info.json | tee $TEMP_FILE

  cp $TEMP_FILE pkg-info.json
}

function changelog_for_release {
  grep -v '^<!-- markdownlint-configure-file .* -->$' UNRELEASED-CHANGELOG.md
}

function update_changelog {
  echo "Updating changelog"
  local TEMP_CHANGELOG=$(mktemp)
  TEMP_FILES+=($TEMP_CHANGELOG)
  (
    echo
    echo "## $NEW_VERSION ($RELEASE_DATE)"
    echo
    changelog_for_release
  ) > $TEMP_CHANGELOG

  # Include changelog entry after marker
  sed -i "/split-marker/ r $TEMP_CHANGELOG" CHANGELOG.md
  # Remove content from UNRELEASED-CHANGELOG
  sed -i '/markdownlint-configure-file/!d' UNRELEASED-CHANGELOG.md
}

function update_gh_workflows {
  echo "Update Github workflows"
  sed -i \
    "s;\(uses: FirelightFlagboy/gh-actions-workflows-docker-services\)/\(.*\)@.*;\1/\2@v$NEW_VERSION;" \
    .github/workflows/docker-build-publish.yml \
    .github/workflows/update-pkg-info.yml
}

function commit_file_for_release {
  echo "Commit release"
  git add \
    pkg-info.json \
    Cargo.toml Cargo.lock \
    CHANGELOG.md UNRELEASED-CHANGELOG.md \
    .github/workflows/docker-build-publish.yml \
    .github/workflows/update-pkg-info.yml \
    $SCRIPTDIR/build.dockerfile
  git commit --signoff $COMMIT_SIGN_ARGS -m "Prepare for release $NEW_VERSION"
}

function create_tag {
  echo "Create tag"
  git tag $TAG_SIGN_ARGS "v$NEW_VERSION" --message="Release version $NEW_VERSION"
}

function push_change {
  echo "Push change"
  git push --atomic origin main "v$NEW_VERSION"
}

function create_release_with_artifact {
  echo "Create github release"

  if [ $SKIP_ARTIFACT_UPLOAD -ne 0 ]; then
    echo "SKIP_ARTIFACT_UPLOAD set, skipping artifact upload"
    local ASSETS=""
  else
    cp target/release/pkg-info-updater /tmp/pkg-info-updater-$OS-$ARCH
    local ASSETS="/tmp/pkg-info-updater-$OS-$ARCH"
  fi

  gh release create "v$NEW_VERSION" \
    --title "$NEW_VERSION" \
    --draft \
    --notes-file $1 \
    --latest \
    --verify-tag \
    $ASSETS
}

build_new_version_for_bin

add_new_version_to_pkg_file

RELEASE_BLOB=$(mktemp)
TEMP_FILES+=($RELEASE_BLOB)

echo "Creating changelog for release"
changelog_for_release > $RELEASE_BLOB

update_changelog

update_gh_workflows

if [ $SKIP_RELEASE_CREATION -ne 0 ]; then
  echo "SKIP_RELEASE_CREATION set, skipping release creation"
  echo
  echo -e "\x1b[1mThe release would have used this notes:\x1b[0m"
  cat $RELEASE_BLOB
  exit 0
fi

commit_file_for_release

create_tag

push_change

create_release_with_artifact $RELEASE_BLOB
