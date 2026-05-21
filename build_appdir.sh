#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
APPDIR="$SCRIPT_DIR/AppDir"

VERSION=$(grep -m1 '^version' "$SCRIPT_DIR/Cargo.toml" | sed -E 's/.*=\s*"([^"]+)".*/\1/')
EXEC="irate_goose"
APPID="dev.barafu_albino.irate-goose"

echo "Building AppDir for version $VERSION"

rm -rf "$APPDIR"
mkdir -p "$APPDIR"

# Copy binary
mkdir -p "$APPDIR/usr/bin"
cp "$SCRIPT_DIR/target/release/$EXEC" "$APPDIR/usr/bin/$APPID"

# Copy icons
mkdir -p "$APPDIR/usr/share/icons/hicolor/64x64/apps"
cp "$SCRIPT_DIR/data/IrateGoose64.png" "$APPDIR/usr/share/icons/hicolor/64x64/apps/barafu-irategoose.png"

mkdir -p "$APPDIR/usr/share/icons/hicolor/48x48/apps"
cp "$SCRIPT_DIR/data/IrateGoose48.png" "$APPDIR/usr/share/icons/hicolor/48x48/apps/barafu-irategoose.png"

mkdir -p "$APPDIR/usr/share/icons/hicolor/256x256/apps"
cp "$SCRIPT_DIR/data/IrateGoose256.png" "$APPDIR/usr/share/icons/hicolor/256x256/apps/barafu-irategoose.png"

ln -s "usr/share/icons/hicolor/256x256/apps/barafu-irategoose.png" "$APPDIR/.DirIcon"
ln -s "usr/share/icons/hicolor/256x256/apps/barafu-irategoose.png" "$APPDIR/barafu-irategoose.png"

# Generate .desktop file from template
mkdir -p "$APPDIR/usr/share/applications"
sed -e "s/{VERSION}/$VERSION/g" -e "s/{EXEC}/$APPID/g" \
    "$SCRIPT_DIR/data/barafu-irategoose.desktop.template" \
    > "$APPDIR/usr/share/applications/$APPID.desktop"

ln -s "usr/share/applications/$APPID.desktop" "$APPDIR/$APPID.desktop"

# Copy AppStream metainfo
mkdir -p "$APPDIR/usr/share/metainfo"
cp "$SCRIPT_DIR/data/$APPID.appdata.xml" "$APPDIR/usr/share/metainfo/$APPID.appdata.xml"

# Create AppRun
cat > "$APPDIR/AppRun" << 'EOF'
#!/bin/bash
exec $APPDIR/usr/bin/dev.barafu_albino.irate-goose $@
EOF
chmod +x "$APPDIR/AppRun"

echo "AppDir created at $APPDIR"
