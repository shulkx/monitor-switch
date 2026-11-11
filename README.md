# MonitorSwitch

在托盘里一键切换显示器输入源，代替显示器背后的按钮和厂商的大体积软件。通过 DDC/CI 控制显示器，支持大部分外接显示器。

## 使用

- 左键托盘图标：打开面板，点磁贴切换。
- 右键托盘图标：菜单里直接选接口；设置；退出。
- Ctrl+Alt+M：在本机和另一台电脑之间来回切（在设置里可以改）。
- 设置里可以给每个接口起名字、录直达快捷键。

装在每一台共用这台显示器的电脑上。显示器菜单里需要打开 DDC/CI。

## 开发

```bash
npm install
npm run tauri dev
cargo test --manifest-path src-tauri/Cargo.toml
npm run tauri build
```

设置文件：`%APPDATA%\MonitorSwitch\config.json`。

设计：`docs/superpowers/specs/2026-11-12-monitorswitch-tray-design.md`
