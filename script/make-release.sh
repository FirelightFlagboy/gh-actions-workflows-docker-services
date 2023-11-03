set -eu -o pipefail

SCRIPTDIR=${SCRIPTDIR:=$(dirname $(realpath -s "$0"))}
ROOTDIR=${ROOTDIR:=$(realpath -s "$SCRIPTDIR/../../..")}

NEW_VERSION=${1:?Missing new version}

function build_new_version_for_bin {
  sed -i "s/^version = \".*\"$/version = \"$NEW_VERSION\"/" Cargo.toml

  cargo build --release
}

function add_new_version_to_pkg_file {
  local ARCH=amd64
  local OS=linux
  local FILENAME=pkg-info-updater-$OS-$ARCH
  local DOWNLOAD_URL=https://github.com/FirelightFlagboy/gh-actions-workflows-docker-services/releases/download/v$NEW_VERSION/pkg-info-updater-linux-amd64
  local SHA512SUM=$(sha512sum target/release/pkg-info-updater | cut -f 1 -d' ')
  local TEMP_FILE=$(mktemp)
  trap "[ -f $TEMP_FILE ] && rm -f $TEMP_FILE" EXIT INT

  jq \
    ".versions[\"$NEW_VERSION\"] = {\"$ARCH\":{\"filename\":\"$FILENAME\",\"download_url\":\"$DOWNLOAD_URL\",\"digest\":\"sha512:$SHA512SUM\"}} | .latest_version = \"$NEW_VERSION\"" \
    pkg-info.json > $TEMP_FILE

  mv $TEMP_FILE pkg-info.json
}

function commit_file_for_release {
  git add pkg-info.json Cargo.toml Cargo.lock
  git commit --signoff --gpg-sign -m "Prepare for release $NEW_VERSION"
}

function create_tag {
  git tag --sign "v$NEW_VERSION" --message="Release version $NEW_VERSION"
}

function push_change {
  git push --atomic origin main "v$NEW_VERSION"
}

function create_release_with_artifact {
  cp target/release/pkg-info-updater /tmp/pkg-info-updater-$OS-$ARCH

  gh release create "v$NEW_VERSION" \
    --title "$NEW_VERSION" \
    --draft \
    --generate-notes \
    --latest \
    --verify-tag \
    "/tmp/pkg-info-updater-$OS-$ARCH"
}

build_new_version_for_bin

add_new_version_to_pkg_file

commit_file_for_release

create_tag

push_change

create_release_with_artifact
