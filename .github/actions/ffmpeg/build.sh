#!/usr/bin/env bash
# Builds the small ffmpeg that comes with the programs: it reads an RTSP stream (H.264 / H.265 / MJPEG) and writes JPEG pictures to a
# pipe. LGPL only (no GPL parts, no non-free ones), static, no TLS, no assembly (no nasm needed). Writes to $OUT: the program,
# COPYING.LGPLv2.1 and FFMPEG-NOTICE.txt. For Windows it is cross-compiled on Linux (TARGET=windows, mingw-w64).
set -euo pipefail

VER=7.1.1
SHA256=733984395e0dbbe5c046abda2dc49a5544e7e0e1e2366bba849222ae9e3a03b1
URL="https://ffmpeg.org/releases/ffmpeg-${VER}.tar.xz"
OUT="${OUT:?OUT is the folder for the result}"
WORK="$(mktemp -d)"
mkdir -p "$OUT"

curl -sSLf "$URL" -o "$WORK/ffmpeg.tar.xz"
echo "${SHA256}  $WORK/ffmpeg.tar.xz" | shasum -a 256 -c -
tar -xf "$WORK/ffmpeg.tar.xz" -C "$WORK"
cd "$WORK/ffmpeg-${VER}"

FLAGS=(
  --disable-everything --disable-autodetect --disable-doc --disable-debug
  --disable-ffplay --disable-ffprobe --disable-x86asm
  --enable-static --disable-shared --enable-network --enable-small
  --enable-protocol=tcp,udp,rtp,pipe,file
  --enable-demuxer=rtsp,rtp,sdp,mjpeg
  --enable-muxer=mjpeg
  --enable-decoder=h264,hevc,mjpeg,mpeg4
  --enable-encoder=mjpeg
  --enable-parser=h264,hevc,mjpeg,mpeg4video
  --enable-filter=fps,scale,format,null,buffer,buffersink,anull,abuffer,abuffersink
)
EXE=ffmpeg
if [ "${TARGET:-}" = windows ]; then
  FLAGS+=(--enable-cross-compile --target-os=mingw32 --arch=x86_64 --cross-prefix=x86_64-w64-mingw32- --extra-ldflags=-static)
  EXE=ffmpeg.exe
fi

./configure "${FLAGS[@]}" > "$WORK/configure.log" 2>&1 || { tail -40 "$WORK/configure.log"; exit 1; }
grep -q "License: LGPL" "$WORK/configure.log" || { echo "the build is not LGPL"; exit 1; }
make -j"$(getconf _NPROCESSORS_ONLN 2>/dev/null || echo 2)" ffmpeg > "$WORK/make.log" 2>&1 || { tail -40 "$WORK/make.log"; exit 1; }

cp "$EXE" "$OUT/$EXE"
cp COPYING.LGPLv2.1 "$OUT/COPYING.LGPLv2.1"
cat > "$OUT/FFMPEG-NOTICE.txt" <<NOTICE
This program comes with FFmpeg ${VER} (https://ffmpeg.org), a separate program that it runs to read the video stream of a camera.
FFmpeg is licensed under the GNU Lesser General Public License, version 2.1 or later (COPYING.LGPLv2.1 is next to this file).

It was built from the unmodified source ${URL}
(sha256 ${SHA256}) with:
./configure ${FLAGS[*]}

The recipe is in https://github.com/michalsarna/baseCreator/tree/main/.github/actions/ffmpeg . You can replace the ffmpeg program
with another one (the program looks for it next to itself first, then in the PATH).
NOTICE
echo "built $OUT/$EXE"
