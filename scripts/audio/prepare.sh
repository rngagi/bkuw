#!/usr/bin/env bash
# Build only the audio codecs needed by bkuw. Never use a host FFmpeg binary.
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/../.." && pwd)
case "$(uname -s)" in
  Darwin) TARGET=aarch64-apple-darwin; test "$(uname -m)" = arm64 ;;
  MINGW*|MSYS*) TARGET=x86_64-pc-windows-msvc ;;
  *) echo "Audio packaging supports macOS Apple Silicon and Windows x64." >&2; exit 1 ;;
esac
SOURCES="$ROOT/.cache/audio-sources"
BUILD="$ROOT/.cache/audio-build/$TARGET"
PREFIX="$BUILD/install"
OUTPUT="$ROOT/src-tauri/resources/audio"
mkdir -p "$SOURCES" "$BUILD" "$PREFIX" "$OUTPUT/sources"
sha256() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{print $1}'
  else
    shasum -a 256 "$1" | awk '{print $1}'
  fi
}
fetch() {
  local name="$1" url="$2" expected="$3"
  if [ ! -f "$SOURCES/$name" ]; then curl --fail --location --retry 3 "$url" -o "$SOURCES/$name"; fi
  test "$(sha256 "$SOURCES/$name")" = "$expected" || { echo "Audio source SHA-256 mismatch" >&2; exit 1; }
}
fetch ffmpeg-8.0.1.tar.xz https://ffmpeg.org/releases/ffmpeg-8.0.1.tar.xz 05ee0b03119b45c0bdb4df654b96802e909e0a752f72e4fe3794f487229e5a41
fetch lame-3.100.tar.gz https://downloads.sourceforge.net/project/lame/lame/3.100/lame-3.100.tar.gz ddfe36cab873794038ae2c1210557ad34857a4b6bdc515785d1da9e175b1da1e
if [ ! -d "$BUILD/lame-3.100" ]; then tar -xf "$SOURCES/lame-3.100.tar.gz" -C "$BUILD"; fi
if [ ! -d "$BUILD/ffmpeg-8.0.1" ]; then tar -xf "$SOURCES/ffmpeg-8.0.1.tar.xz" -C "$BUILD"; fi
JOBS=4
if [ "$TARGET" = aarch64-apple-darwin ]; then
  export CC="${CC:-clang}"
  export MACOSX_DEPLOYMENT_TARGET=11.0
  JOBS=$(sysctl -n hw.logicalcpu)
fi
cd "$BUILD/lame-3.100"
if [ -f Makefile ]; then make clean; fi
# LAME's old config.sub predates Apple Silicon; explicit host skips that detection.
LAME_HOST=()
if [ "$TARGET" = aarch64-apple-darwin ]; then LAME_HOST=(--host=aarch64-apple-darwin); fi
./configure --prefix="$PREFIX" --disable-shared --enable-static --disable-frontend --disable-decoder --disable-asm "${LAME_HOST[@]}"
make -j"$JOBS"
make install
cd "$BUILD/ffmpeg-8.0.1"
if [ -f ffbuild/config.mak ]; then make clean; fi
STATIC_FLAGS=""
if [ "$TARGET" = x86_64-pc-windows-msvc ]; then STATIC_FLAGS="-static"; fi
./configure --prefix="$PREFIX" --disable-everything --disable-autodetect --disable-network \
  --disable-shared --enable-static --disable-doc --disable-debug --disable-x86asm \
  --disable-ffplay --enable-ffmpeg --enable-ffprobe --enable-libmp3lame \
  --extra-cflags="-I$PREFIX/include" --extra-ldflags="-L$PREFIX/lib $STATIC_FLAGS" \
  --enable-protocol=file,pipe --enable-demuxer=wav,mp3,mov,aac,flac,ogg,aiff \
  --enable-decoder=mp3,mp3float,aac,alac,flac,vorbis,opus,pcm_s8,pcm_u8,pcm_s16le,pcm_s16be,pcm_s24le,pcm_s24be,pcm_s32le,pcm_s32be,pcm_f32le,pcm_f32be,pcm_f64le,pcm_f64be,pcm_alaw,pcm_mulaw \
  --enable-parser=mpegaudio,aac,flac,opus,vorbis --enable-encoder=libmp3lame \
  --enable-muxer=mp3 --enable-filter=aresample,aformat,anull,atrim
make -j"$JOBS"
SUFFIX=""
if [ "$TARGET" = x86_64-pc-windows-msvc ]; then SUFFIX=.exe; fi
cp "ffmpeg$SUFFIX" "ffprobe$SUFFIX" "$OUTPUT/"
cp COPYING.LGPLv2.1 "$OUTPUT/FFmpeg-LICENSE.txt"
cp "$BUILD/lame-3.100/COPYING" "$OUTPUT/LAME-LICENSE.txt"
# Exact sources plus this build recipe permit rebuilding/relinking the tools.
cp "$SOURCES/ffmpeg-8.0.1.tar.xz" "$SOURCES/lame-3.100.tar.gz" "$OUTPUT/sources/"
cp "$ROOT/scripts/audio/prepare.sh" "$OUTPUT/sources/"
cat > "$OUTPUT/manifest.json" <<MANIFEST
{
  "target": "$TARGET",
  "ffmpeg": "8.0.1",
  "lame": "3.100",
  "recipeSha256": "$(sha256 "$OUTPUT/sources/prepare.sh")",
  "sha256": {
    "ffmpeg$SUFFIX": "$(sha256 "$OUTPUT/ffmpeg$SUFFIX")",
    "ffprobe$SUFFIX": "$(sha256 "$OUTPUT/ffprobe$SUFFIX")"
  }
}
MANIFEST
