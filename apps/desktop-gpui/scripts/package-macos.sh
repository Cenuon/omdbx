#!/bin/sh
# Build a self-contained macOS .app, zip, and dmg for one architecture.
set -eu

ROOT=$(CDPATH= cd -- "$(dirname "$0")/../../.." && pwd)
VERSION=$(awk -F '"' '/^version = / { print $2; exit }' "$ROOT/apps/desktop-gpui/Cargo.toml")
REQUESTED_ARCH=${1:-$(uname -m)}

case "$REQUESTED_ARCH" in
  arm64|aarch64)
    TARGET=aarch64-apple-darwin
    ARCH_NAME=arm64
    ARTIFACT_SUFFIX=-arm64
    ;;
  x64|x86_64)
    TARGET=x86_64-apple-darwin
    ARCH_NAME=x64
    ARTIFACT_SUFFIX=
    ;;
  *)
    echo "unsupported architecture: $REQUESTED_ARCH (expected arm64 or x64)" >&2
    exit 1
    ;;
esac

if [ -n "${CARGO_TARGET_DIR:-}" ]; then
  TARGET_ROOT=$CARGO_TARGET_DIR
else
  TARGET_ROOT=$ROOT/target
fi

if [ "$TARGET" = "$(rustc -vV | awk '/^host:/ { print $2 }')" ]; then
  TARGET_FLAG=
  BACKEND_BIN=$TARGET_ROOT/release/dbx-web
  GPUI_BIN=$ROOT/apps/desktop-gpui/target/release/omdbx-gpui
  if [ -n "${CARGO_TARGET_DIR:-}" ]; then
    GPUI_BIN=$TARGET_ROOT/release/omdbx-gpui
  fi
else
  TARGET_FLAG="--target $TARGET"
  BACKEND_BIN=$TARGET_ROOT/$TARGET/release/dbx-web
  GPUI_BIN=$ROOT/apps/desktop-gpui/target/$TARGET/release/omdbx-gpui
  if [ -n "${CARGO_TARGET_DIR:-}" ]; then
    GPUI_BIN=$TARGET_ROOT/$TARGET/release/omdbx-gpui
  fi
fi

if [ "${SKIP_BUILD:-0}" != "1" ]; then
  cargo build --release --manifest-path "$ROOT/Cargo.toml" -p dbx-web \
    --no-default-features \
    --features duckdb-sidecar,dynamodb,mq-admin,dbx-core/sqlite-bundled \
    $TARGET_FLAG
  cargo build --release --manifest-path "$ROOT/apps/desktop-gpui/Cargo.toml" $TARGET_FLAG
fi

for binary in "$BACKEND_BIN" "$GPUI_BIN"; do
  if [ ! -f "$binary" ]; then
    echo "missing build output: $binary" >&2
    exit 1
  fi
done

OUT_DIR=$ROOT/dist/desktop-gpui
APP_DIR=$OUT_DIR/OMDBX.app
CONTENTS_DIR=$APP_DIR/Contents
RESOURCES_DIR=$CONTENTS_DIR/Resources
DMG_ROOT=$OUT_DIR/dmg-root
ZIP_NAME=OMDBX-$VERSION${ARTIFACT_SUFFIX}-mac.zip
DMG_NAME=OMDBX-$VERSION${ARTIFACT_SUFFIX}.dmg

rm -rf "$APP_DIR" "$DMG_ROOT"
mkdir -p "$CONTENTS_DIR/MacOS" "$RESOURCES_DIR"

cp "$GPUI_BIN" "$CONTENTS_DIR/MacOS/OMDBX"
cp "$BACKEND_BIN" "$RESOURCES_DIR/dbx-web"
cp "$ROOT/apps/desktop-gpui/assets/AppIcon.icns" "$RESOURCES_DIR/AppIcon.icns"
cp "$ROOT/LICENSE" "$RESOURCES_DIR/LICENSE"
cp "$ROOT/NOTICE" "$RESOURCES_DIR/NOTICE"
chmod +x "$CONTENTS_DIR/MacOS/OMDBX" "$RESOURCES_DIR/dbx-web"

cat > "$CONTENTS_DIR/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleDevelopmentRegion</key>
  <string>en</string>
  <key>CFBundleDisplayName</key>
  <string>OMDBX</string>
  <key>CFBundleExecutable</key>
  <string>OMDBX</string>
  <key>CFBundleIconFile</key>
  <string>AppIcon</string>
  <key>CFBundleIdentifier</key>
  <string>io.omdbx.desktop</string>
  <key>CFBundleInfoDictionaryVersion</key>
  <string>6.0</string>
  <key>CFBundleName</key>
  <string>OMDBX</string>
  <key>CFBundlePackageType</key>
  <string>APPL</string>
  <key>CFBundleShortVersionString</key>
  <string>$VERSION</string>
  <key>CFBundleVersion</key>
  <string>$VERSION</string>
  <key>LSMinimumSystemVersion</key>
  <string>12.0</string>
  <key>NSHighResolutionCapable</key>
  <true/>
  <key>NSPrincipalClass</key>
  <string>NSApplication</string>
</dict>
</plist>
EOF

printf 'APPL????' > "$CONTENTS_DIR/PkgInfo"
touch "$APP_DIR"
xattr -cr "$APP_DIR"
codesign --force --deep --sign - "$APP_DIR"

rm -f "$OUT_DIR/$ZIP_NAME" "$OUT_DIR/$DMG_NAME"
ditto -c -k --norsrc --keepParent "$APP_DIR" "$OUT_DIR/$ZIP_NAME"

mkdir -p "$DMG_ROOT"
cp -R "$APP_DIR" "$DMG_ROOT/"
ln -s /Applications "$DMG_ROOT/Applications"
hdiutil create -quiet -volname "OMDBX" -srcfolder "$DMG_ROOT" -ov -format UDZO "$OUT_DIR/$DMG_NAME"
rm -rf "$DMG_ROOT"

echo "packaged OMDBX $VERSION for $ARCH_NAME"
echo "  $APP_DIR"
echo "  $OUT_DIR/$ZIP_NAME"
echo "  $OUT_DIR/$DMG_NAME"
