:: Stable is probably already installed, and you might not want to upgrade it right now, so check first.
@rustup toolchain list | findstr msvc | findstr stable >NUL 2>NUL && goto :skip-install-stable
rustup install stable-pc-windows-msvc
:skip-install-stable

:: Used for CI
rustup install 1.24.0-pc-windows-msvc
rustup install nightly-pc-windows-msvc
rustup install nightly-pc-windows-gnu
:: 64-bit GNU doesn't like making 32-bit executables (recognized "--large-address-aware" flag)
rustup install nightly-i686-pc-windows-gnu
rustup target add --toolchain nightly-pc-windows-msvc i686-pc-windows-msvc

:: Not used by CI, but you might want these for the .vscode examples
rustup target add --toolchain 1.24.0-pc-windows-msvc i686-pc-windows-msvc
rustup target add --toolchain stable-pc-windows-msvc i686-pc-windows-msvc
