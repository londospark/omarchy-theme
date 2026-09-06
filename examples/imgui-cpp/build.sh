#!/usr/bin/env bash
# Build + (optionally) run the ImGui C++ demo.
#   ./build.sh          -> headless smoke binary + run the smoke test
#   ./build.sh test     -> run the smoke test
#   ./build.sh run      -> build with GLFW/OpenGL3 backends and run live
set -euo pipefail
cd "$(dirname "$0")/../.."
VENDOR=build/imgui-vendor
[[ -f $VENDOR/imgui.cpp ]] || { echo "run the vendoring step (see examples/README.md)" >&2; exit 1; }

IMGUI_SRCS="$VENDOR/imgui.cpp $VENDOR/imgui_draw.cpp $VENDOR/imgui_tables.cpp $VENDOR/imgui_widgets.cpp"

build_smoke() {
  mkdir -p build
  g++ -std=c++17 -O1 -Iinclude -I$VENDOR $IMGUI_SRCS examples/imgui-cpp/main.cpp -o build/imgui-cpp-smoke
}

if [[ ${1:-test} == test ]]; then
  build_smoke
  ./build/imgui-cpp-smoke --test
elif [[ $1 == run ]]; then
  command -v pkg-config >/dev/null && pkg-config --exists glfw3 || { echo "glfw3 dev package required" >&2; exit 1; }
  build() {
    g++ -std=c++17 -O1 -DOMARCHY_DEMO_GUI -Iinclude -I$VENDOR \
      $($=pkg-config --cflags glfw3) \
      $IMGUI_SRCS $VENDOR/backends/imgui_impl_glfw.cpp $VENDOR/backends/imgui_impl_opengl3.cpp \
      examples/imgui-cpp/main.cpp examples/imgui-cpp/demo_glfw.cpp \
      -o build/imgui-cpp-demo $($=pkg-config --libs glfw3) -lGL
  }
  mkdir -p $VENDOR/backends
  for f in imgui_impl_glfw imgui_impl_opengl3; do
    [[ -f $VENDOR/backends/$f.cpp ]] || curl -sL -o $VENDOR/backends/$f.cpp https://raw.githubusercontent.com/ocornut/imgui/docking/backends/$f.cpp
  done
  build
  ./build/imgui-cpp-demo
else
  echo "usage: build.sh [test|run]" >&2; exit 2
fi
