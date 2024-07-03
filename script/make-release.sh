set -eu -o pipefail

OS=linux
ARCH=amd64

SCRIPTDIR=${SCRIPTDIR:=$(dirname $(realpath -s "$0"))}
ROOTDIR=${ROOTDIR:=$(realpath -s "$SCRIPTDIR/../../..")}
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

  cargo build --release
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
    pkg-info.json > $TEMP_FILE

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
    .github/workflows/update-pkg-info.yml
  git commit --signoff --gpg-sign -m "Prepare for release $NEW_VERSION"
}

function create_tag {
  echo "Create tag"
  git tag --sign "v$NEW_VERSION" --message="Release version $NEW_VERSION"
}

function push_change {
  echo "Push change"
  git push --atomic origin main "v$NEW_VERSION"
}

function create_release_with_artifact {
  echo "Create github release"
  cp target/release/pkg-info-updater /tmp/pkg-info-updater-$OS-$ARCH

  gh release create "v$NEW_VERSION" \
    --title "$NEW_VERSION" \
    --draft \
    --notes-file $1 \
    --latest \
    --verify-tag \
    "/tmp/pkg-info-updater-$OS-$ARCH"
}

build_new_version_for_bin

add_new_version_to_pkg_file

RELEASE_BLOB=$(mktemp)
TEMP_FILES+=($RELEASE_BLOB)

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
