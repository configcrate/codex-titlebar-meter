# Codex Titlebar Meter v0.1.3

小而专一的额度状态更新。

- 剩余额度不超过 20% 时，进度条自动变黄。
- 剩余额度不超过 10% 时，进度条自动变红。
- 连续两分钟没有成功刷新时，保留最后一次数据，并用灰色状态明确提示“数据已过期 · 正在重连”。
- 恢复连接后自动回到实时额度，无需点击或重启。
- 双额度模式为两个重置标签保留足够空间，单额度模式仍保持原来的紧凑宽度。
- 不增加账户操作、通知弹窗或额外联网服务。

Built by [ConfigCrate](https://configcrate.com/).

---

A focused quota-status update.

- The progress bar turns amber at 20% remaining.
- The progress bar turns red at 10% remaining.
- After two minutes without a successful refresh, the last snapshot is retained and a gray `Usage stale · reconnecting` state appears.
- Live quota display returns automatically after reconnection, with no click or restart required.
- Dual-window mode leaves enough room for both reset labels, while single-window mode keeps its original compact width.
- Adds no account actions, notification popups, or extra network services.

Built by [ConfigCrate](https://configcrate.com/).
