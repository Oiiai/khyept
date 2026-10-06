#!/usr/bin/env bash
# Khyept 构建脚本
#
# 主要用途：在 Git Bash（PortableGit）下绕开链接器冲突。
# PortableGit 自带的 /usr/bin/link.exe 会抢占 MSVC 的 link.exe，
# 导致链接失败（LNK1181 找不到 kernel32.lib）。此脚本显式指定
# MSVC 工具链与 Windows SDK 路径，保证在 Git Bash 下也能正确链接。
#
# 注意：LIB / INCLUDE 必须是 Windows 风格路径且用分号分隔，
# 因此统一用 cygpath -w 转换，不能用 POSIX 路径。
#
# 在 PowerShell、cmd 或 Linux/macOS 下直接用 cargo 即可，无需本脚本。
set -euo pipefail

# 定位 Visual Studio 的 MSVC 工具链，兼容 2022 / 2019 / 18 等不同安装布局
find_msvc_root() {
    local found=()
    local pattern base
    for pattern in \
        "/c/Program Files/Microsoft Visual Studio"/*/*/VC/Tools/MSVC \
        "/c/Program Files (x86)/Microsoft Visual Studio"/*/*/VC/Tools/MSVC \
        "/c/BuildTools/VC/Tools/MSVC"
    do
        [ -d "$pattern" ] && found+=("$pattern")
    done
    [ ${#found[@]} -eq 0 ] && return 1
    # 取版本号最大的一个
    printf '%s\n' "${found[@]}" | sort -V | tail -1
}

MSVC_ROOT="$(find_msvc_root || true)"
SDK_ROOT="/c/Program Files (x86)/Windows Kits/10"

if [ -z "$MSVC_ROOT" ] || [ ! -d "$SDK_ROOT/Lib" ]; then
    # 找不到 MSVC 或 SDK，说明不需要特殊处理，直接用 cargo
    cd "$(dirname "$0")"
    exec cargo "$@"
fi

MSVC_VER="$(ls "$MSVC_ROOT" | sort -V | tail -1)"
SDK_VER="$(ls "$SDK_ROOT/Lib" | sort -V | tail -1)"

MSVC_BIN="$MSVC_ROOT/$MSVC_VER/bin/Hostx64/x64"
MSVC_LIB="$MSVC_ROOT/$MSVC_VER/lib/x64"
SDK_UM="$SDK_ROOT/Lib/$SDK_VER/um/x64"
SDK_UCRT="$SDK_ROOT/Lib/$SDK_VER/ucrt/x64"

export PATH="$MSVC_BIN:$PATH"
export LIB="$(cygpath -w "$MSVC_LIB");$(cygpath -w "$SDK_UM");$(cygpath -w "$SDK_UCRT")"
export INCLUDE="$(cygpath -w "$MSVC_ROOT/$MSVC_VER/include");$(cygpath -w "$SDK_ROOT/Include/$SDK_VER/ucrt");$(cygpath -w "$SDK_ROOT/Include/$SDK_VER/um");$(cygpath -w "$SDK_ROOT/Include/$SDK_VER/shared")"

cd "$(dirname "$0")"
exec cargo "$@"