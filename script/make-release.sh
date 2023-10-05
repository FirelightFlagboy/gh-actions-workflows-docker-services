set -eu -o pipefail

SCRIPTDIR=${SCRIPTDIR:=$(dirname $(realpath -s "$0"))}
ROOTDIR=${ROOTDIR:=$(realpath -s "$SCRIPTDIR/../../..")}

NEW_VERSION=${1:?Missing new version}

sed -i "s/^version = \".*\"$/version = \"$NEW_VERSION\"/" Cargo.toml

cargo build --release

ARCH=amd64
OS=linux

SHA512SUM=$(sha512sum target/release/pkg-info-updater | cut -f 1 -d' ')

FILENAME=pkg-info-updater-$OS-$ARCH
DOWNLOAD_URL=https://github.com/FirelightFlagboy/gh-actions-workflows-docker-services/releases/download/v$NEW_VERSION/pkg-info-updater-linux-amd64
TEMP_FILE=$(mktemp)
trap "[ -f $TEMP_FILE ] && rm -f $TEMP_FILE" EXIT INT

jq \
  ".versions[\"$NEW_VERSION\"] = {\"$ARCH\":{\"filename\":\"$FILENAME\",\"download_url\":\"$DOWNLOAD_URL\",\"digest\":\"sha512:$SHA512SUM\"}}" \
  pkg-info.json > $TEMP_FILE

mv $TEMP_FILE pkg-info.json

git add pkg-info.json Cargo.toml Cargo.lock

git commit --signoff --gpg-sign -m "Prepare for release $NEW_VERSION"
git tag --sign "v$NEW_VERSION" --message="Release version $NEW_VERSION"

git push --atomic origin main "v$NEW_VERSION"

cp target/release/pkg-info-updater /tmp/pkg-info-updater-$OS-$ARCH

gh release create "v$NEW_VERSION" \
  --title "$NEW_VERSION" \
  --draft \
  --generate-notes \
  --latest \
  --verify-tag \
  "/tmp/pkg-info-updater-$OS-$ARCH"
