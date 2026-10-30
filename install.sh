#!/usr/bin/env bash
# install.sh —— 从源码把 TinyDock + tclock 安装到 /usr/local
#   * 二进制 -> /usr/local/bin/{tinydock,tclock}
#   * 菜单项 -> /usr/share/applications/{tinydock,tclock}.desktop
#   * 登录自启 -> ~/.config/autostart/{tinydock,tclock}.desktop
#   * tclock 源码位置可用环境变量 TCLOCK_SRC 覆盖
set -euo pipefail

HERE=$(cd $(dirname "$0") && pwd)
APP=tinydock
BIN=$HERE/target/release/$APP
DESKIN=$HERE/$APP.desktop.in
TCLOCK_SRC=${TCLOCK_SRC:-$HERE/tclock}
TCLOCK_BIN=$TCLOCK_SRC/target/release/tclock
TCLOCK_DESKIN=$HERE/tclock.desktop.in
DESK=$HERE/$APP.desktop
TCDESK=$HERE/tclock.desktop

if [ ! -x $BIN ]; then
  echo '==> 构建 dock'
  (cd $HERE && cargo build --release)
fi
if [ ! -x $TCLOCK_BIN ]; then
  echo '==> 构建 tclock'
  (cd $TCLOCK_SRC && cargo build --release)
fi

# 生成最终 desktop（@BINDIR@ -> /usr/local/bin）
sed 's|@BINDIR@|/usr/local/bin|g' $DESKIN > $DESK
sed 's|@BINDIR@|/usr/local/bin|g' $TCLOCK_DESKIN > $TCDESK

if [ $(id -u) != 0 ]; then
  SUDO=sudo
else
  SUDO=
fi

$SUDO install -m755 $BIN /usr/local/bin/$APP
$SUDO install -m755 $TCLOCK_BIN /usr/local/bin/tclock
$SUDO install -m644 $DESK /usr/share/applications/$APP.desktop
$SUDO install -m644 $TCDESK /usr/share/applications/tclock.desktop

# 登录自启（两者都启用）
mkdir -p $HOME/.config/autostart
install -m644 $DESK $HOME/.config/autostart/$APP.desktop
install -m644 $TCDESK $HOME/.config/autostart/tclock.desktop

echo '==> 安装完成。运行:' $APP '与 tclock'
