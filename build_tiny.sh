#!/usr/bin/env bash
# expects cygwin to call windows cargo, and cygwin upx.

# get your triple with `rustc -vV

TRIPLE="x86_64-pc-windows-msvc"

RUSTFLAGS="-Zlocation-detail=none -Zfmt-debug=none" cargo +nightly build \
  -Z build-std=std,panic_abort \
  -Z build-std-features=panic_immediate_abort \
  -Z build-std-features=optimize_for_size \
  --target $TRIPLE --release

upx --best --brute target/$TRIPLE/release/ccontext.exe