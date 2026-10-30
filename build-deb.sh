#!/usr/bin/env bash
# build-deb.sh —— 把 TinyDock + tclock 打包成 .deb
# 运行期依赖用 dpkg-shlibdeps 自动推导（兼容 Ubuntu 24.04 的 t64 改名）。
set -euo pipefail

APP=tinydock
VERSION=${1:-1.0}
ARCH=$(dpkg --print-architecture 2>/dev/null || echo amd64)
HERE=$(cd $(dirname "$0") && pwd)
BIN=$HERE/target/release/$APP
DESKIN=$HERE/$APP.desktop.in
# tclock 源码位置（可用环境变量 TCLOCK_SRC 覆盖）
TCLOCK_SRC=${TCLOCK_SRC:-/home/u/workspace/tclock}
TCLOCK_BIN=$TCLOCK_SRC/target/release/tclock
TCLOCK_DESKIN=$HERE/tclock.desktop.in
OUT=$HERE/${APP}_${VERSION}_${ARCH}.deb

# 1) 确保有 release 二进制（dock + tclock）
if [ ! -x $BIN ]; then
  echo '==> 构建 dock'
  (cd $HERE && cargo build --release)
fi
if [ ! -x $TCLOCK_BIN ]; then
  echo '==> 构建 tclock'
  (cd $TCLOCK_SRC && cargo build --release)
fi

# 2) 准备目录树
STAGE=$(mktemp -d)
DPKGCTX=$(mktemp -d)
trap 'rm -rf "$STAGE" "$DPKGCTX"' EXIT
mkdir -p $STAGE/usr/bin $STAGE/usr/share/applications $STAGE/etc/xdg/autostart $STAGE/DEBIAN

install -m755 $BIN $STAGE/usr/bin/$APP
install -m755 $TCLOCK_BIN $STAGE/usr/bin/tclock

# 3) desktop 文件（@BINDIR@ -> /usr/bin），同时放入应用菜单与系统级自启
for pair in $DESKIN:tinydock.desktop $TCLOCK_DESKIN:tclock.desktop; do
  src=${pair%%:*}
  dst=${pair##*:}
  sed 's|@BINDIR@|/usr/bin|g' $src > $STAGE/usr/share/applications/$dst
  install -m644 $STAGE/usr/share/applications/$dst $STAGE/etc/xdg/autostart/$dst
done
chmod 644 $STAGE/usr/share/applications/*.desktop

# 4) dpkg-shlibdeps 自动推导依赖（dock 的依赖；tclock 的 GL/X11 为 dlopen，另行补充）
echo '==> 推导共享库依赖 (dpkg-shlibdeps)'
mkdir -p $DPKGCTX/debian
cat > $DPKGCTX/debian/control <<'CTRL'
Source: tinydock
Section: x11
Priority: optional
Maintainer: tinydock <tinydock@example.com>

Package: tinydock
Architecture: any
Depends: ${shlibs:Depends}
Description: placeholder
CTRL

DEPS=$(cd $DPKGCTX && dpkg-shlibdeps -O $BIN $TCLOCK_BIN 2>/dev/null | sed 's/^shlibs:Depends=//')
if [ -z "$DEPS" ]; then
  echo '!! dpkg-shlibdeps 失败，回退到 dpkg -S 方式'
  DEPS=$(
    for b in $BIN $TCLOCK_BIN; do
      for lib in $(ldd $b | awk '/=>/{print $3}'); do
        dpkg -S $lib 2>/dev/null | head -1 | cut -d: -f1
      done
    done | sort -u | grep -v '^$' | paste -sd ', '
  )
fi
# tclock 通过 dlopen 加载 GL/XKB/X11，dpkg-shlibdeps 看不到，这里补显式依赖
# 这些包不在 time_t 改名(t64)名单内，跨 22.04/24.04 安全
TCLOCK_EXTRA='libgl1, libxkbcommon0, libx11-6'
DEPS=$(echo "$DEPS, $TCLOCK_EXTRA" | tr ',' '\n' | sed 's/^ *//' | sort -u | paste -sd ',' | sed 's/,/, /g')
echo '    Depends:' $DEPS

cat > $STAGE/DEBIAN/control <<EOF
Package: $APP
Version: $VERSION
Section: x11
Priority: optional
Architecture: $ARCH
Depends: $DEPS
Maintainer: tinydock <tinydock@example.com>
Description: TinyDock 轻量 dock 与 tclock 时钟
 TinyDock：Rust + GTK3 实现的底部 dock，悬停弹性放大、窗口指示圆点、右键菜单。
 tclock：Rust + eframe 实现的桌面模拟时钟小组件。
 两者均随登录自动启动。
EOF

# 5) 权限修正后打包
find $STAGE -type f -exec chmod 644 {} +
chmod 755 $STAGE/usr/bin/$APP $STAGE/usr/bin/tclock

echo '==> 生成' $OUT
dpkg-deb --build --root-owner-group $STAGE $OUT
echo '==> 完成:' $OUT
