#!/bin/sh
# Builds a universal (Apple Silicon + Intel) native Metal build/KeyCat.app with zero WebKit.
# Usage: sh native/mac/build.sh
set -eu
cd "$(dirname "$0")/../.."

APP=build/KeyCat.app
rm -rf "$APP" build/arch
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources" build/arch

# 1. Build universal keycat-ffi static library (arm64 + x86_64)
rustup target add aarch64-apple-darwin x86_64-apple-darwin >/dev/null 2>&1 || true
cargo build --manifest-path native/Cargo.toml --release --target aarch64-apple-darwin -p keycat-ffi
cargo build --manifest-path native/Cargo.toml --release --target x86_64-apple-darwin -p keycat-ffi

# 2. Compile Metal shaders into default.metallib (also ship source as fallback)
cp native/mac/Shaders.metal "$APP/Contents/Resources/Shaders.metal"
if xcrun -f metal >/dev/null 2>&1 && xcrun -f metallib >/dev/null 2>&1; then
    xcrun metal -O2 -mmacosx-version-min=13.0 -c native/mac/Shaders.metal -o build/arch/Shaders.air
    xcrun metallib build/arch/Shaders.air -o "$APP/Contents/Resources/default.metallib"
fi

# 3. Compile Swift + Metal native shell linked directly against libkeycat_ffi.a (zero WebKit)
for pair in "arm64:aarch64-apple-darwin" "x86_64:x86_64-apple-darwin"; do
    arch="${pair%%:*}"
    target="${pair#*:}"
    xcrun swiftc -O -swift-version 5 -target "$arch-apple-macosx13.0" \
        -import-objc-header native/ffi/include/keycat_ffi.h \
        -framework Cocoa -framework Metal -framework QuartzCore -framework CoreText \
        "native/target/$target/release/libkeycat_ffi.a" \
        -o "build/arch/KeyCat-$arch" \
        native/mac/*.swift
done

lipo -create build/arch/KeyCat-arm64 build/arch/KeyCat-x86_64 -output "$APP/Contents/MacOS/KeyCat"
cp native/mac/Info.plist "$APP/Contents/Info.plist"
sh native/mac/sign.sh "$APP"
echo "built native Metal $APP"
