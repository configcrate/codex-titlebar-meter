# Codex Titlebar Meter v0.1.4

修复 Codex 更新后缓存持续增长的问题。此前每个 Codex 桌面版本都会留下约 300 MB 的 `codex.exe` 副本，长期使用可能占用多个 GiB。

- 启动时自动清理本程序生成的旧版 CLI 缓存，仅保留当前版本。
- 正在运行、被 Windows 锁定的旧副本会在后续启动时重试清理。
- 仅处理符合 Codex 包命名、且只含程序缓存文件的目录；设置和其他文件不会被删除。
- 不改变额度读取、标题栏显示和现有安装方式。

Built by [ConfigCrate](https://configcrate.com/).

---

Fixes unbounded cache growth after Codex Desktop updates. Previous releases kept a roughly 300 MB `codex.exe` copy for every desktop version, potentially consuming several GiB over time.

- Automatically removes old CLI caches created by the meter on startup, keeping the current version.
- Retries cleanup later if Windows still locks an executable in use.
- Touches only Codex package-named cache directories containing generated executable files; preferences and other files are preserved.
- Quota reading, title-bar display, and installation behavior are unchanged.

Built by [ConfigCrate](https://configcrate.com/).
