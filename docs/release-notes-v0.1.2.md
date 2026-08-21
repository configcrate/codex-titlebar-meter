# Codex Titlebar Meter v0.1.2

修复新版 Codex Desktop 更新后无法读取用量的问题。

- 适配新版 `codex app-server` 的启动参数。
- 继续使用只读沙盒，不请求操作权限。
- 支持新版只返回单个额度窗口的情况，例如 `1周额度 96%`。
- 不修改 Codex 文件，现有安装设置保持不变。

Built by [ConfigCrate](https://configcrate.com/).

---

Fixes usage reading after recent Codex Desktop updates.

- Uses the current `codex app-server` approval argument.
- Keeps the session read-only and never requests action approval.
- Supports current responses that contain a single quota window, such as `1 week quota 96%`.
- Continues to modify no Codex files and preserves existing settings.

Built by [ConfigCrate](https://configcrate.com/).
