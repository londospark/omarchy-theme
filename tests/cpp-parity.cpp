// cpp-parity.cpp — dump the C++ header's resolved palette as key<TAB>value
// (oracle `--all` format) for cross-language parity checks:
//   g++ -std=c++17 -Iinclude -Ibuild/imgui-vendor tests/cpp-parity.cpp -o build/cpp-parity
//   ./build/cpp-parity <colors.toml> [light]
// The check-parity.sh script runs it against every conformance vector.
#include <omarchy/imgui.hpp>

int main(int argc, char** argv) {
    if (argc < 2) return 2;
    bool marker = argc > 2 && std::string(argv[2]) == "light";
    omarchy::Palette p = omarchy::Palette::FromFile(argv[1], marker);
    for (auto& [k, v] : p.entries) {
        if (v.empty()) continue; // vectors drop empties
        printf("%s\t%s\n", k.c_str(), v.c_str());
    }
    return 0;
}
