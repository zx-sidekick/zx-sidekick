# static-crt.cmake: a CMake toolchain file for the Windows release build, which links the C
# runtime into the program (-C target-feature=+crt-static). SDL3, built from source by CMake,
# must use the same runtime: the static, non-debug one Rust links whatever the profile.
# .github/workflows/release.yml hands it to the cmake crate as CMAKE_TOOLCHAIN_FILE_<target>.
set(CMAKE_MSVC_RUNTIME_LIBRARY MultiThreaded)
