# Smart Screen

一个位于 macOS 菜单栏的温湿度托盘应用。

菜单栏按钮显示当前温度与湿度，点击后展开以下信息：

- 温度
- 湿度
- 数据源连接状态
- 退出应用

## 运行

先用模拟数据检查界面：

```bash
cargo run --offline -- --demo
```

读取 Pico 的 USB 串口：

```bash
cargo run --offline -- --serial /dev/cu.usbmodem14101
```

从 Pico W 的 HTTP JSON 接口读取：

```bash
cargo run --offline -- --url http://192.168.4.1/data
```

不传参数时，应用会自动查找 `/dev/cu.usbmodem*` 或 JDY 串口。

## 数据格式

串口和 HTTP 均支持以下 JSON：

```json
{"temperature":24.6,"humidity":57}
```

Pico 当前的 USB 调试输出也可以直接识别：

```text
environment: 24 57
```

## 构建 macOS DMG

本地构建并打包当前架构：

```bash
cargo build --release
scripts/package-macos.sh
```

产物位于：

```text
dist/SmartScreen-0.1.0-macos-<arch>.dmg
```

GitHub Actions 工作流位于 `.github/workflows/build-dmg.yml`。它会构建 Intel
和 Apple Silicon 两个版本，合并成 universal 二进制，然后生成带 Applications
快捷方式的 DMG，并作为 workflow artifact 上传。

DMG 当前使用 ad-hoc 签名。公开分发时仍建议配置 Apple Developer ID 证书和
notarization。
