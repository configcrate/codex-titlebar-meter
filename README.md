# Codex Titlebar Meter

无需点击账户菜单，直接在 Codex Desktop 标题栏查看剩余额度。

Codex Titlebar Meter 是一个原生 Windows 伴生程序。它把 Codex 返回的真实额度窗口、剩余百分比和重置日期显示在 Codex 标题栏空白区域，并随 Codex 移动、缩放、最大化、最小化和退出。

## 特点

- 零点击：使用 Codex 时始终可见。
- 自动跟随：只附着在 `OpenAI.Codex` 桌面窗口，不覆盖其他应用。
- 真实数据：通过本机 `codex app-server` 读取，不估算 Token。
- 不要 API Key：不读取、不复制、不保存登录令牌。
- 不修改 Codex：不注入 DLL，不改应用文件，不受普通 Codex 更新覆盖。
- 按实际额度显示：账号没有返回短周期窗口时，不会伪造一个进度条。
- 轻量原生：Rust + Win32/GDI，无 WebView、Electron 或后台 Windows 服务。
- 易于移除：按用户安装，无需管理员权限，并出现在 Windows“已安装的应用”中。

## 操作

- 拖动用量条：移动 Codex 窗口。
- 双击用量条：最大化或还原 Codex。
- 点击右侧彩色方块：切换蓝色、绿色和紫色主题。
- 右键用量条：同样可以切换颜色。

颜色设置保存在：

```text
%LOCALAPPDATA%\ConfigCrate\CodexTitlebarMeter\settings.json
```

## 安装

从 Releases 下载 Windows x64 ZIP，解压后运行：

```powershell
powershell -ExecutionPolicy Bypass -File .\install.ps1
```

安装程序会：

1. 复制程序到 `%LOCALAPPDATA%\Programs\CodexTitlebarMeter`。
2. 注册当前用户登录时启动。
3. 立即开始监视 Codex。
4. 在 Windows“已安装的应用”中注册卸载入口。

不安装也可以直接运行 `codex-titlebar-meter.exe` 作为便携版。

## 卸载

可以从 Windows“已安装的应用”卸载，或者运行：

```powershell
powershell -ExecutionPolicy Bypass -File .\uninstall.ps1
```

保留颜色设置是默认行为。彻底清除缓存和设置：

```powershell
powershell -ExecutionPolicy Bypass -File .\uninstall.ps1 -PurgeSettings
```

## 数据与隐私

程序从 Store 版 Codex 安装目录复制其本地 `codex.exe` 到自己的用户缓存，再以只读沙盒参数启动 `codex app-server`。Codex 自己管理现有登录状态；本程序只接收额度百分比、周期和重置时间。

没有遥测，没有第三方服务器，也不会向 ConfigCrate 上传数据。

## 当前范围

- Windows 10/11 x64
- OpenAI Codex Desktop（Microsoft Store 版）
- ChatGPT 管理的 Codex 额度窗口
- 深色 Codex 标题栏

多窗口、浅色主题和 ARM64 将在实际需求出现后再扩展。

## 开发

需要 Rust stable：

```powershell
cargo test
cargo run
```

构建发布包：

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\build-release.ps1
```

## English

Codex Titlebar Meter shows your real Codex quota directly in the empty area of the Codex Desktop title bar. There is no tray click, dashboard, API key, credential copy, or modification to Codex files.

The overlay follows the Codex window and reads quota snapshots from the local `codex app-server`. Click the small color swatch to change the accent. Install per user with `install.ps1`, or run the executable as a portable app.

## License

MIT. Independent project; not affiliated with or endorsed by OpenAI.

Built by [ConfigCrate](https://configcrate.com/).

