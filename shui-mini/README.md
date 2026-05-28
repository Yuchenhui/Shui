# Shui Mini

极简喝水提醒：纯托盘 + 系统通知，**没有 WebView，没有打包浏览器**。

## 与原版 Shui 的区别

| | Shui (Tauri) | Shui Mini |
|---|---|---|
| 架构 | Rust + WebView2 + React | 纯 Rust |
| Windows 内存占用 | ~130 MB（5 个 webview 进程）| ~10 MB（单进程）|
| 二进制大小 | ~10 MB | ~2 MB |
| 用户界面 | 全屏精美弹窗 + 设置页 | 系统托盘 + 原生通知 |
| 配置方式 | 图形设置页 | 托盘菜单 + TOML 文件 |

## 已支持功能

- 自定义提醒间隔（30/45/60/90 分钟，或编辑配置文件设置任意值）
- 工作时段限定（默认 09:00–18:00，可改）
- 仅工作日（默认开）
- **锁屏自动暂停**（Windows 已实现，macOS/Linux 暂不支持）
- 开机自启动
- 每日饮水杯数统计

## 托盘菜单

```
● 间隔 45 分钟          ← 状态行
今日已喝 3 杯
───
我喝了一杯 +1
暂停提醒
立即提醒一次
重置倒计时
───
提醒间隔 ›
  ✓ 30 分钟
    45 分钟
    60 分钟
    90 分钟
仅工作日提醒 ✓
开机自启
───
打开配置文件…
退出
```

## 配置文件路径

- Windows: `%APPDATA%\shui-mini\config.toml`
- macOS: `~/Library/Application Support/shui-mini/config.toml`
- Linux: `~/.config/shui-mini/config.toml`

统计数据：`%LOCALAPPDATA%\shui-mini\stats.json`（Windows）等。

## 构建

```bash
cargo build --release -p shui-mini
```

产物在 `target/release/shui-mini`（Linux/macOS）或 `shui-mini.exe`（Windows）。

### Linux 依赖

构建需要 `libgtk-3-dev`、`libayatana-appindicator3-dev`、`libxdo-dev`。运行时需要 GTK3 + AppIndicator。

### 内存目标

Windows 上稳定运行时内存占用应在 10~15 MB 范围。

## GitHub Actions 跨平台构建

仓库附带了一个跨平台构建工作流模板：`shui-mini/ci/build-shui-mini.yaml`。
**首次启用需要把它挪到 `.github/workflows/` 下**（GitHub App 凭据无 `workflows` 权限，
所以不能由 Claude 直接推送到那个位置，需要你本地拷贝一次）：

```bash
mkdir -p .github/workflows
cp shui-mini/ci/build-shui-mini.yaml .github/workflows/
git add .github/workflows/build-shui-mini.yaml
git commit -m "ci(shui-mini): enable cross-platform build workflow"
git push
```

之后这个 workflow 由 `.github/workflows/` 里的副本驱动，`shui-mini/ci/` 下保留
模板仅作参考（修改后记得同步两边）。

工作流提供：

| 触发方式 | 行为 |
|---|---|
| 推 `mini-v*` 标签（如 `mini-v0.1.0`）| 4 个平台构建 + 自动创建 GitHub Release |
| PR 涉及 `shui-mini/` | 仅 CI 验证编译 |
| 手动 `workflow_dispatch` | 产 Actions artifact，不发 Release |

构建矩阵：

| 平台 | Runner | 产物 |
|---|---|---|
| Windows x64 | `windows-latest` (MSVC) | `shui-mini-windows-x64.zip` |
| macOS ARM64 | `macos-latest` | `shui-mini-macos-arm64.tar.gz` |
| macOS x64   | `macos-13` (Intel) | `shui-mini-macos-x64.tar.gz` |
| Linux x64   | `ubuntu-22.04` | `shui-mini-linux-x64.tar.gz` |

