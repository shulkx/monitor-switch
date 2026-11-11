# MonitorSwitch 核心（Windows）实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 做出一个能在 Windows 上日常使用的 MonitorSwitch：托盘图标、磁贴面板、屏幕提示、快捷键、设置窗口、本机接口自动识别、开机自启。

**Architecture:** Tauri 2 应用。Rust 后端里，一个 `MonitorService` 工作线程独占显示器通信，串行执行切换和轮询，通过回调把事件交给应用层。应用层负责托盘、窗口、快捷键和设置文件。界面是三个原生 TypeScript 页面（面板、屏幕提示、设置），通过 Tauri 命令和事件与后端通信。Windows 上的 DDC/CI 和连接类型检测直接调用系统 API（dxva2、QueryDisplayConfig）。

**Tech Stack:** Rust 1.99、Tauri 2.12、tauri-plugin-global-shortcut 2.4、tauri-plugin-autostart 2.7、tauri-plugin-single-instance 2.5、serde；Vite + TypeScript（不用 UI 框架）。

**规格文档:** `docs/superpowers/specs/2026-11-12-monitorswitch-tray-design.md`

---

## 分期

规格里的内容分 4 个计划实现。每个计划结束时，软件都能运行和测试：

| 计划 | 内容 | 对应规格章节 |
|---|---|---|
| **1（本计划）** | 核心 Windows 版：托盘、面板、OSD、快捷键、设置、本机识别、读接口列表的崩溃保护、自启、单实例 | 3.1–3.3、3.5、4、5.1–5.4、6.1–6.4、6.6、6.7、7 |
| 2 | 学习接口窗口、新设备提示、锁屏/休眠时暂停轮询 | 3.4、3.6、5.2（新设备、暂停） |
| 3 | 局域网互相发现 | 5.5、6.5、3.5 中的局域网开关 |
| 4 | macOS / Linux 的显示器后端与本机识别、GitHub Actions、安装包 | 4（ddc-hi）、5.6、7（CI）、8 |

**与规格的一处差异（已同步修改规格）：** 规格原写 Windows 也用 `ddc-hi`。实际查看 `ddc-hi` 0.4.1 源码后发现两个问题：它在 Windows 上默认同时启用 NVIDIA 后端，这台机器的 GTX 980 会让同一台显示器出现两次；它的 Windows 后端也不提供 EDID，没法和连接类型对上号。所以 Windows 版直接调用系统 API，`ddc-hi` 留给计划 4 的 macOS / Linux。

## 前置条件

本机已具备：Rust 1.99、Node 24 / npm 11、Visual Studio 2022 生成工具、WebView2 运行时、git。

项目根目录（下文的"仓库根目录"）就是本文件所在仓库的根，即包含 `docs/` 的那一层。所有命令都在仓库根目录执行，除非另有说明。路径较深时，Windows 可能报"Filename too long"，仓库已设置 `git config core.longpaths true`。

## 文件结构

```
package.json, tsconfig.json, vite.config.ts, .gitignore, README.md
flyout.html, osd.html, settings.html          三个页面入口
src/shared/api.ts                             后端命令和事件的类型化封装、HTML 转义
src/shared/hotkey.ts                          键盘事件 → 快捷键字符串；美化显示
src/shared/icons.ts                           线条图标 SVG
src/shared/theme.css                          颜色、字体等设计变量
src/flyout/main.ts, flyout.css                托盘面板（磁贴）
src/osd/main.ts, osd.css                      屏幕提示
src/settings/main.ts, settings.css            设置窗口
src-tauri/Cargo.toml, build.rs, tauri.conf.json, capabilities/default.json, icons/
src-tauri/src/main.rs                         可执行入口
src-tauri/src/lib.rs                          模块声明、插件、run()
src-tauri/src/model.rs                        MonitorKey、ConnectionKind、MonitorInfo
src-tauri/src/ddc/mod.rs
src-tauri/src/ddc/caps.rs                     解析接口列表字符串；输入值命名与分类
src-tauri/src/ddc/backend.rs                  DisplayBackend trait
src-tauri/src/ddc/fake.rs                     测试用假后端（仅 cfg(test)）
src-tauri/src/local_port.rs                   本机接口识别规则（纯函数）
src-tauri/src/toggle.rs                       来回切换的目标选择、默认接口对（纯函数）
src-tauri/src/config.rs                       设置结构、读写、崩溃恢复
src-tauri/src/monitor_service.rs              工作线程：切换、确认、轮询、事件
src-tauri/src/view.rs                         把状态和设置合成界面用的 View（纯函数）
src-tauri/src/hotkeys.rs                      快捷键绑定、冲突检查、注册表
src-tauri/src/platform/mod.rs                 按平台选择后端
src-tauri/src/platform/stub.rs                非 Windows 的占位后端（计划 4 替换）
src-tauri/src/platform/win/mod.rs
src-tauri/src/platform/win/ffi.rs             Win32 函数和结构体的原始声明
src-tauri/src/platform/win/topology.rs        QueryDisplayConfig：连接类型和设备路径
src-tauri/src/platform/win/backend.rs         WinBackend：dxva2 读写输入、读接口列表；是否笔记本
src-tauri/src/app/mod.rs                      setup()：组装各部件
src-tauri/src/app/state.rs                    AppState、ConfigPrefs
src-tauri/src/app/actions.rs                  切换、来回切换、快捷键、服务事件处理、自启
src-tauri/src/app/flyout.rs                   面板显示/隐藏/定位
src-tauri/src/app/osd.rs                      屏幕提示
src-tauri/src/app/tray.rs                     托盘图标、悬停提示、右键菜单
src-tauri/src/app/windows.rs                  创建窗口
src-tauri/src/commands.rs                     暴露给界面的 Tauri 命令
```

---

### Task 1: 搭项目骨架

**Files:**
- Create: `package.json`、`tsconfig.json`（从官方模板复制）、`src-tauri/icons/*`、`src-tauri/build.rs`（从模板复制）
- Create: `.gitignore`、`vite.config.ts`、`flyout.html`、`osd.html`、`settings.html`
- Create: `src/flyout/main.ts`、`src/osd/main.ts`、`src/settings/main.ts`（占位）
- Create: `src-tauri/Cargo.toml`、`src-tauri/tauri.conf.json`、`src-tauri/capabilities/default.json`、`src-tauri/src/main.rs`、`src-tauri/src/lib.rs`

- [ ] **Step 1: 在仓库外生成官方模板**

在仓库根目录的上一级执行（不要在仓库里执行）：

```bash
cd ..
npm create tauri-app@4.7.5 -- ms-scaffold --template vanilla-ts --manager npm --identifier dev.monitorswitch.app --yes
```

预期：生成 `ms-scaffold/` 文件夹。如果该版本不接受这些参数，就直接运行 `npm create tauri-app@4.7.5`，依次选：项目名 `ms-scaffold`、标识符 `dev.monitorswitch.app`、语言 TypeScript / JavaScript、包管理器 npm、界面模板 Vanilla、风格 TypeScript。

- [ ] **Step 2: 复制需要的模板文件，删除模板**

```bash
cp ms-scaffold/package.json ms-scaffold/tsconfig.json monitor-switch/
mkdir -p monitor-switch/src-tauri
cp -r ms-scaffold/src-tauri/icons ms-scaffold/src-tauri/build.rs monitor-switch/src-tauri/
rm -rf ms-scaffold
cd monitor-switch
```

（如果仓库文件夹不叫 `monitor-switch`，把上面的 `monitor-switch` 换成实际名字。）

- [ ] **Step 3: 改 package.json 的名字和版本**

把 `package.json` 里的 `"name"` 改为 `"monitor-switch"`，`"version"` 改为 `"0.1.0"`。确认 `scripts` 里有 `"dev": "vite"`、`"build": "tsc && vite build"`、`"tauri": "tauri"`，没有就补上。

- [ ] **Step 4: 写 `.gitignore`**

```gitignore
node_modules/
dist/
src-tauri/target/
src-tauri/gen/
*.log
```

- [ ] **Step 5: 写 `vite.config.ts`（三个页面入口）**

```ts
import { defineConfig } from "vite";
import { fileURLToPath } from "node:url";

const host = process.env.TAURI_DEV_HOST;
const page = (name: string) => fileURLToPath(new URL(`${name}.html`, import.meta.url));

export default defineConfig({
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host ? { protocol: "ws", host, port: 1421 } : undefined,
    watch: { ignored: ["**/src-tauri/**"] },
  },
  build: {
    rollupOptions: {
      input: { flyout: page("flyout"), osd: page("osd"), settings: page("settings") },
    },
  },
});
```

- [ ] **Step 6: 写三个页面入口和占位脚本**

`flyout.html`：

```html
<!doctype html>
<html lang="zh-CN">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>MonitorSwitch</title>
    <link rel="stylesheet" href="/src/shared/theme.css" />
    <link rel="stylesheet" href="/src/flyout/flyout.css" />
  </head>
  <body>
    <main id="panel" class="panel" aria-label="MonitorSwitch"></main>
    <script type="module" src="/src/flyout/main.ts"></script>
  </body>
</html>
```

`osd.html`：

```html
<!doctype html>
<html lang="zh-CN">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>MonitorSwitch 提示</title>
    <link rel="stylesheet" href="/src/shared/theme.css" />
    <link rel="stylesheet" href="/src/osd/osd.css" />
  </head>
  <body>
    <div id="osd" class="osd" role="status" aria-live="polite"></div>
    <script type="module" src="/src/osd/main.ts"></script>
  </body>
</html>
```

`settings.html`：

```html
<!doctype html>
<html lang="zh-CN">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>MonitorSwitch 设置</title>
    <link rel="stylesheet" href="/src/shared/theme.css" />
    <link rel="stylesheet" href="/src/settings/settings.css" />
  </head>
  <body>
    <div id="app"></div>
    <script type="module" src="/src/settings/main.ts"></script>
  </body>
</html>
```

占位文件（后面的任务会替换）：`src/flyout/main.ts`、`src/osd/main.ts`、`src/settings/main.ts` 内容都是：

```ts
export {};
```

`src/shared/theme.css`、`src/flyout/flyout.css`、`src/osd/osd.css`、`src/settings/settings.css` 先建成空文件。

删除模板带来的 `index.html`、`src/main.ts`、`src/styles.css`、`src/assets/`（如果 Step 2 没有复制它们则跳过）。

- [ ] **Step 7: 写 `src-tauri/Cargo.toml`**

```toml
[package]
name = "monitor-switch"
version = "0.1.0"
description = "Switch monitor inputs from the tray"
edition = "2021"

[lib]
name = "monitor_switch_lib"
crate-type = ["staticlib", "cdylib", "rlib"]

[build-dependencies]
tauri-build = { version = "2", features = [] }

[dependencies]
tauri = { version = "2", features = ["tray-icon"] }
tauri-plugin-global-shortcut = "2"
tauri-plugin-autostart = "2"
tauri-plugin-single-instance = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"

[dev-dependencies]
tempfile = "3"
```

- [ ] **Step 8: 写 `src-tauri/tauri.conf.json`**

```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "MonitorSwitch",
  "version": "0.1.0",
  "identifier": "dev.monitorswitch.app",
  "build": {
    "beforeDevCommand": "npm run dev",
    "devUrl": "http://localhost:1420",
    "beforeBuildCommand": "npm run build",
    "frontendDist": "../dist"
  },
  "app": {
    "windows": [],
    "security": { "csp": null }
  },
  "bundle": {
    "active": true,
    "targets": ["nsis"],
    "icon": [
      "icons/32x32.png",
      "icons/128x128.png",
      "icons/128x128@2x.png",
      "icons/icon.icns",
      "icons/icon.ico"
    ],
    "windows": {
      "nsis": { "installMode": "currentUser" }
    }
  }
}
```

- [ ] **Step 9: 写 `src-tauri/capabilities/default.json`**

```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "default",
  "description": "Permissions for the MonitorSwitch windows",
  "windows": ["flyout", "osd", "settings"],
  "permissions": ["core:default"]
}
```

- [ ] **Step 10: 写 `src-tauri/src/main.rs` 和最小的 `src-tauri/src/lib.rs`**

`src-tauri/src/main.rs`：

```rust
// Hide the console window in release builds on Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    monitor_switch_lib::run()
}
```

`src-tauri/src/lib.rs`：

```rust
pub fn run() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("error while running MonitorSwitch");
}
```

- [ ] **Step 11: 安装依赖并验证前后端都能构建**

```bash
npm install
npm uninstall @tauri-apps/plugin-opener
npm run build
cargo build --manifest-path src-tauri/Cargo.toml
```

预期：`npm run build` 生成 `dist/flyout.html`、`dist/osd.html`、`dist/settings.html`；`cargo build` 最后一行是 `Finished`（第一次要几分钟）。`npm uninstall` 如果提示没有这个包，忽略即可。

- [ ] **Step 12: Commit**

```bash
git add -A
git commit -m "chore: scaffold Tauri 2 app with three pages"
```

---

### Task 2: 基础类型和接口列表解析

**Files:**
- Create: `src-tauri/src/model.rs`
- Create: `src-tauri/src/ddc/mod.rs`
- Create: `src-tauri/src/ddc/caps.rs`
- Modify: `src-tauri/src/lib.rs`（加模块声明）

- [ ] **Step 1: 写 `model.rs`**

```rust
use serde::{Deserialize, Serialize};

/// Stable identifier of a monitor on this computer, derived from its device path.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct MonitorKey(pub String);

/// How this computer is physically connected to a monitor, as reported by the OS.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ConnectionKind {
    Hdmi,
    DisplayPort,
    /// DisplayPort carried over USB4 / Thunderbolt.
    UsbTunnel,
    Other,
}

/// A monitor the backend can talk to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MonitorInfo {
    pub key: MonitorKey,
    pub label: String,
    pub connection: ConnectionKind,
}
```

- [ ] **Step 2: 写 `ddc/mod.rs`**

```rust
pub mod caps;
```

- [ ] **Step 3: 在 `lib.rs` 顶部加模块声明**

```rust
pub mod ddc;
pub mod model;
```

- [ ] **Step 4: 写会失败的测试（`ddc/caps.rs`，先只有测试和空函数签名）**

```rust
//! Parse the DDC/CI capabilities string a monitor reports, and name input values.

/// VCP feature code for "Input Select".
pub const VCP_INPUT_SELECT: u8 = 0x60;
/// Input value most vendors (Dell included) use for USB-C. Not part of MCCS.
pub const USB_C: u16 = 0x1B;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Capabilities {
    pub model: Option<String>,
    /// Values the monitor lists for VCP 0x60, in the order it lists them.
    pub inputs: Vec<u16>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PortClass {
    Dp,
    Hdmi,
    UsbC,
    Other,
}

pub fn parse(_raw: &str) -> Capabilities {
    Capabilities::default()
}

pub fn input_label(_value: u16) -> String {
    String::new()
}

pub fn port_class(_value: u16) -> PortClass {
    PortClass::Other
}

/// Capabilities string in the format Dell monitors report.
#[cfg(test)]
pub(crate) const DELL_STYLE_CAPS: &str = "(prot(monitor)type(LCD)model(U2723QE)cmds(01 02 03 07 0C E3 F3)vcp(02 04 05 08 10 12 14(01 04 05 06 08 09 0B 0C) 16 18 1A 52 60(0F 11 1B) AA(01 02 04) AC AE B2 B6 C6 C8 C9 D6(01 04 05) DC(00 03 05) DF E0 E1 E2(00 1D 01 02 04 0E 12 14 23 24 27) F0(00 0C 0F 10 11 31 32 34 35) F1 F2 FD)mswhql(1)asset_eep(40)mccs_ver(2.1))";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_model_and_inputs() {
        let caps = parse(DELL_STYLE_CAPS);
        assert_eq!(caps.model.as_deref(), Some("U2723QE"));
        assert_eq!(caps.inputs, vec![0x0F, 0x11, 0x1B]);
    }

    #[test]
    fn tolerates_case_and_spacing() {
        assert_eq!(parse("(VCP(10 60 ( 0f  11 ) 62))").inputs, vec![0x0F, 0x11]);
    }

    #[test]
    fn skips_value_lists_of_other_features() {
        assert_eq!(parse("(vcp(14(60 01) 60(12)))").inputs, vec![0x12]);
    }

    #[test]
    fn ignores_the_vcpname_section() {
        assert_eq!(parse("(vcpname(60(Input)) vcp(60(11)))").inputs, vec![0x11]);
    }

    #[test]
    fn missing_or_broken_strings_give_nothing() {
        assert_eq!(parse(""), Capabilities::default());
        assert!(parse("(vcp(10 12))").inputs.is_empty());
        assert!(parse("(vcp(60(0F 11").inputs.is_empty());
    }

    #[test]
    fn names_inputs() {
        assert_eq!(input_label(0x0F), "DP");
        assert_eq!(input_label(0x10), "DP 2");
        assert_eq!(input_label(0x11), "HDMI");
        assert_eq!(input_label(0x12), "HDMI 2");
        assert_eq!(input_label(USB_C), "USB-C");
        assert_eq!(input_label(0x90), "0x90");
    }

    #[test]
    fn classifies_inputs() {
        assert_eq!(port_class(0x10), PortClass::Dp);
        assert_eq!(port_class(0x11), PortClass::Hdmi);
        assert_eq!(port_class(USB_C), PortClass::UsbC);
        assert_eq!(port_class(0x01), PortClass::Other);
    }
}
```

- [ ] **Step 5: 运行测试，确认失败**

Run: `cargo test --manifest-path src-tauri/Cargo.toml caps`
Expected: FAIL，`parses_model_and_inputs`、`names_inputs` 等断言不通过。

- [ ] **Step 6: 实现 `parse`、`input_label`、`port_class`**

把 Step 4 里三个空函数替换为：

```rust
pub fn parse(raw: &str) -> Capabilities {
    let model = section(raw, "model")
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let inputs = section(raw, "vcp")
        .map(|vcp| feature_values(vcp, "60"))
        .unwrap_or_default();
    Capabilities { model, inputs }
}

/// Short name shown in the UI for a VCP 0x60 value.
pub fn input_label(value: u16) -> String {
    match value {
        0x01 => "VGA".into(),
        0x02 => "VGA 2".into(),
        0x03 => "DVI".into(),
        0x04 => "DVI 2".into(),
        0x0F => "DP".into(),
        0x10 => "DP 2".into(),
        0x11 => "HDMI".into(),
        0x12 => "HDMI 2".into(),
        USB_C => "USB-C".into(),
        other => format!("0x{other:02X}"),
    }
}

pub fn port_class(value: u16) -> PortClass {
    match value {
        0x0F | 0x10 => PortClass::Dp,
        0x11 | 0x12 => PortClass::Hdmi,
        USB_C => PortClass::UsbC,
        _ => PortClass::Other,
    }
}

/// Text inside `name( ... )`, matching parentheses. `name` must be a whole word,
/// so "vcp" does not match "vcpname".
fn section<'a>(raw: &'a str, name: &str) -> Option<&'a str> {
    let lower = raw.to_ascii_lowercase();
    let needle = format!("{name}(");
    let mut from = 0;
    while let Some(pos) = lower[from..].find(&needle) {
        let at = from + pos;
        let starts_word = at == 0 || {
            let before = lower.as_bytes()[at - 1];
            !before.is_ascii_alphanumeric() && before != b'_'
        };
        if starts_word {
            return balanced(raw, at + name.len()).map(|(inner, _)| inner);
        }
        from = at + needle.len();
    }
    None
}

/// `open` is the index of a '('. Returns the text up to its matching ')' and the index after it.
fn balanced(s: &str, open: usize) -> Option<(&str, usize)> {
    let mut depth = 0usize;
    for (i, &b) in s.as_bytes().iter().enumerate().skip(open) {
        match b {
            b'(' => depth += 1,
            b')' => {
                depth -= 1;
                if depth == 0 {
                    return Some((&s[open + 1..i], i + 1));
                }
            }
            _ => {}
        }
    }
    None
}

/// Inside a `vcp(...)` section, the hex values listed for feature `code`.
fn feature_values(vcp: &str, code: &str) -> Vec<u16> {
    let bytes = vcp.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if !bytes[i].is_ascii_hexdigit() {
            i += 1;
            continue;
        }
        let start = i;
        while i < bytes.len() && bytes[i].is_ascii_hexdigit() {
            i += 1;
        }
        let token = &vcp[start..i];
        let mut j = i;
        while j < bytes.len() && bytes[j].is_ascii_whitespace() {
            j += 1;
        }
        if j < bytes.len() && bytes[j] == b'(' {
            let Some((inner, end)) = balanced(vcp, j) else {
                return Vec::new();
            };
            if token.eq_ignore_ascii_case(code) {
                return inner
                    .split_whitespace()
                    .filter_map(|t| u16::from_str_radix(t, 16).ok())
                    .collect();
            }
            i = end;
        }
    }
    Vec::new()
}
```

- [ ] **Step 7: 运行测试，确认通过**

Run: `cargo test --manifest-path src-tauri/Cargo.toml caps`
Expected: `test result: ok. 7 passed`

- [ ] **Step 8: Commit**

```bash
git add src-tauri/src/model.rs src-tauri/src/ddc src-tauri/src/lib.rs
git commit -m "feat: parse DDC/CI capabilities and name input values"
```

---

### Task 3: 本机接口识别规则

**Files:**
- Create: `src-tauri/src/local_port.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: 在 `lib.rs` 加 `pub mod local_port;`**

- [ ] **Step 2: 写会失败的测试**

`src-tauri/src/local_port.rs`：

```rust
//! Guess which input of a monitor this computer is plugged into (spec 5.3).

use crate::ddc::caps::{port_class, PortClass, USB_C};
use crate::model::ConnectionKind;

/// `inputs` is what the monitor lists for VCP 0x60 (empty when unknown).
/// A user override is applied by the caller, not here.
pub fn guess_local_input(_connection: ConnectionKind, _is_laptop: bool, _inputs: &[u16]) -> Option<u16> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use ConnectionKind::*;

    const DELL: [u16; 3] = [0x0F, 0x11, 0x1B];

    #[test]
    fn desktop_on_displayport_uses_dp() {
        assert_eq!(guess_local_input(DisplayPort, false, &DELL), Some(0x0F));
    }

    #[test]
    fn laptop_on_displayport_uses_usb_c_when_the_monitor_has_it() {
        assert_eq!(guess_local_input(DisplayPort, true, &DELL), Some(0x1B));
    }

    #[test]
    fn laptop_on_displayport_without_usb_c_uses_dp() {
        assert_eq!(guess_local_input(DisplayPort, true, &[0x0F, 0x11]), Some(0x0F));
    }

    #[test]
    fn hdmi_uses_the_lowest_hdmi_value() {
        assert_eq!(guess_local_input(Hdmi, false, &[0x12, 0x0F, 0x11]), Some(0x11));
    }

    #[test]
    fn hdmi_without_hdmi_inputs_gives_none() {
        assert_eq!(guess_local_input(Hdmi, false, &[0x0F]), None);
    }

    #[test]
    fn usb_tunnel_prefers_usb_c() {
        assert_eq!(guess_local_input(UsbTunnel, false, &DELL), Some(0x1B));
    }

    #[test]
    fn unknown_connection_gives_none() {
        assert_eq!(guess_local_input(Other, false, &DELL), None);
    }

    #[test]
    fn without_capabilities_falls_back_to_standard_values() {
        assert_eq!(guess_local_input(DisplayPort, false, &[]), Some(0x0F));
        assert_eq!(guess_local_input(DisplayPort, true, &[]), Some(USB_C));
        assert_eq!(guess_local_input(Hdmi, false, &[]), Some(0x11));
    }
}
```

- [ ] **Step 3: 运行测试，确认失败**

Run: `cargo test --manifest-path src-tauri/Cargo.toml local_port`
Expected: FAIL（除 `unknown_connection_gives_none` 和 `hdmi_without_hdmi_inputs_gives_none` 外都失败）

- [ ] **Step 4: 实现**

把空函数替换为：

```rust
pub fn guess_local_input(connection: ConnectionKind, is_laptop: bool, inputs: &[u16]) -> Option<u16> {
    let lowest = |class: PortClass| inputs.iter().copied().filter(|&v| port_class(v) == class).min();
    let has_usb_c = inputs.contains(&USB_C);
    let unknown = inputs.is_empty();
    match connection {
        ConnectionKind::Hdmi => lowest(PortClass::Hdmi).or(if unknown { Some(0x11) } else { None }),
        ConnectionKind::DisplayPort => {
            if is_laptop && has_usb_c {
                return Some(USB_C);
            }
            if let Some(dp) = lowest(PortClass::Dp) {
                return Some(dp);
            }
            if has_usb_c {
                return Some(USB_C);
            }
            if unknown {
                Some(if is_laptop { USB_C } else { 0x0F })
            } else {
                None
            }
        }
        ConnectionKind::UsbTunnel => {
            if has_usb_c || unknown {
                Some(USB_C)
            } else {
                lowest(PortClass::Dp)
            }
        }
        ConnectionKind::Other => None,
    }
}
```

- [ ] **Step 5: 运行测试，确认通过**

Run: `cargo test --manifest-path src-tauri/Cargo.toml local_port`
Expected: `test result: ok. 8 passed`

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/local_port.rs src-tauri/src/lib.rs
git commit -m "feat: guess which monitor input this computer uses"
```

---

### Task 4: 来回切换逻辑

**Files:**
- Create: `src-tauri/src/toggle.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: 在 `lib.rs` 加 `pub mod toggle;`**

- [ ] **Step 2: 写会失败的测试**

`src-tauri/src/toggle.rs`：

```rust
//! Which input the "toggle" hotkey switches to (spec 5.2).

/// If the monitor is on the first input of the pair, go to the second; otherwise go to the first.
pub fn toggle_target(_current: Option<u16>, pair: [u16; 2]) -> u16 {
    pair[1]
}

/// The pair used when the user has not chosen one: this computer's input plus the
/// first other input that has a name, else the first other input the monitor lists.
pub fn default_pair(_local: Option<u16>, _named: &[u16], _inputs: &[u16]) -> Option<[u16; 2]> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toggles_between_the_two_inputs() {
        assert_eq!(toggle_target(Some(0x0F), [0x0F, 0x1B]), 0x1B);
        assert_eq!(toggle_target(Some(0x1B), [0x0F, 0x1B]), 0x0F);
    }

    #[test]
    fn from_any_other_input_goes_to_the_first() {
        assert_eq!(toggle_target(Some(0x11), [0x0F, 0x1B]), 0x0F);
        assert_eq!(toggle_target(None, [0x0F, 0x1B]), 0x0F);
    }

    #[test]
    fn default_pair_prefers_a_named_input() {
        assert_eq!(default_pair(Some(0x0F), &[0x0F, 0x1B], &[0x0F, 0x11, 0x1B]), Some([0x0F, 0x1B]));
    }

    #[test]
    fn default_pair_falls_back_to_listed_inputs() {
        assert_eq!(default_pair(Some(0x0F), &[], &[0x0F, 0x11, 0x1B]), Some([0x0F, 0x11]));
    }

    #[test]
    fn no_default_pair_without_a_local_input_or_another_input() {
        assert_eq!(default_pair(None, &[0x1B], &[0x0F, 0x1B]), None);
        assert_eq!(default_pair(Some(0x0F), &[], &[0x0F]), None);
    }
}
```

- [ ] **Step 3: 运行测试，确认失败**

Run: `cargo test --manifest-path src-tauri/Cargo.toml toggle`
Expected: FAIL

- [ ] **Step 4: 实现**

```rust
pub fn toggle_target(current: Option<u16>, pair: [u16; 2]) -> u16 {
    if current == Some(pair[0]) {
        pair[1]
    } else {
        pair[0]
    }
}

pub fn default_pair(local: Option<u16>, named: &[u16], inputs: &[u16]) -> Option<[u16; 2]> {
    let local = local?;
    let other = named
        .iter()
        .copied()
        .find(|&v| v != local)
        .or_else(|| inputs.iter().copied().find(|&v| v != local))?;
    Some([local, other])
}
```

- [ ] **Step 5: 运行测试，确认通过**

Run: `cargo test --manifest-path src-tauri/Cargo.toml toggle`
Expected: `test result: ok. 5 passed`

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/toggle.rs src-tauri/src/lib.rs
git commit -m "feat: toggle target and default toggle pair"
```

---

### Task 5: 设置文件

**Files:**
- Create: `src-tauri/src/config.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: 在 `lib.rs` 加 `pub mod config;`**

- [ ] **Step 2: 写结构体和会失败的测试**

`src-tauri/src/config.rs`：

```rust
//! Settings file (spec 5.4).

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::model::MonitorKey;

pub const CONFIG_VERSION: u32 = 1;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Config {
    pub version: u32,
    pub show_osd: bool,
    /// Used by plan 3 (LAN discovery).
    pub lan_discovery: bool,
    pub autostart: bool,
    pub toggle_hotkey: Option<String>,
    pub monitors: BTreeMap<String, MonitorConfig>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            version: CONFIG_VERSION,
            show_osd: true,
            lan_discovery: true,
            autostart: true,
            toggle_hotkey: Some("Ctrl+Alt+M".into()),
            monitors: BTreeMap::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct MonitorConfig {
    pub label: String,
    pub read_capabilities: bool,
    /// True while a capabilities read is in progress. Still true at startup means that
    /// read crashed the app (or Windows), see spec 6.4.
    pub caps_read_pending: bool,
    pub cached_capabilities: Option<String>,
    pub local_port_override: Option<u16>,
    pub toggle_pair: Option<[u16; 2]>,
    /// Keyed by the input value in decimal, because JSON object keys are strings.
    pub ports: BTreeMap<String, PortConfig>,
    /// Used by plan 2 (learn ports).
    pub learned_ports: BTreeMap<String, String>,
}

impl Default for MonitorConfig {
    fn default() -> Self {
        MonitorConfig {
            label: String::new(),
            read_capabilities: true,
            caps_read_pending: false,
            cached_capabilities: None,
            local_port_override: None,
            toggle_pair: None,
            ports: BTreeMap::new(),
            learned_ports: BTreeMap::new(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PortConfig {
    pub name: Option<String>,
    pub hotkey: Option<String>,
    /// Used by plan 2 (new device prompt).
    pub ignored: bool,
}

impl Config {
    pub fn monitor(&self, key: &MonitorKey) -> Option<&MonitorConfig> {
        self.monitors.get(&key.0)
    }

    pub fn monitor_mut(&mut self, key: &MonitorKey) -> &mut MonitorConfig {
        self.monitors.entry(key.0.clone()).or_default()
    }

    /// Turn off automatic capability reads for monitors whose last read never finished,
    /// and return their labels.
    pub fn recover_interrupted_caps_reads(&mut self) -> Vec<String> {
        Vec::new()
    }
}

impl MonitorConfig {
    pub fn port(&self, value: u16) -> Option<&PortConfig> {
        self.ports.get(&value.to_string())
    }

    pub fn port_mut(&mut self, value: u16) -> &mut PortConfig {
        self.ports.entry(value.to_string()).or_default()
    }

    /// Input values that have a name, ascending.
    pub fn named_ports(&self) -> Vec<u16> {
        Vec::new()
    }
}

#[derive(Clone, Debug)]
pub struct ConfigStore {
    path: PathBuf,
}

impl ConfigStore {
    pub fn new(path: PathBuf) -> Self {
        ConfigStore { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn load(&self) -> Config {
        Config::default()
    }

    pub fn save(&self, _cfg: &Config) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> (tempfile::TempDir, ConfigStore) {
        let dir = tempfile::tempdir().unwrap();
        let store = ConfigStore::new(dir.path().join("MonitorSwitch").join("config.json"));
        (dir, store)
    }

    #[test]
    fn defaults_match_the_spec() {
        let cfg = Config::default();
        assert!(cfg.show_osd && cfg.autostart && cfg.lan_discovery);
        assert_eq!(cfg.toggle_hotkey.as_deref(), Some("Ctrl+Alt+M"));
        assert!(MonitorConfig::default().read_capabilities);
    }

    #[test]
    fn missing_file_gives_defaults() {
        let (_dir, store) = store();
        assert_eq!(store.load(), Config::default());
    }

    #[test]
    fn save_then_load_round_trips() {
        let (_dir, store) = store();
        let mut cfg = Config::default();
        cfg.show_osd = false;
        let key = MonitorKey("DEL426A#UID1".into());
        cfg.monitor_mut(&key).port_mut(0x1B).name = Some("公司笔记本".into());
        cfg.monitor_mut(&key).toggle_pair = Some([0x0F, 0x1B]);
        store.save(&cfg).unwrap();
        assert_eq!(store.load(), cfg);
    }

    #[test]
    fn partial_file_fills_missing_fields_with_defaults() {
        let (_dir, store) = store();
        fs::create_dir_all(store.path().parent().unwrap()).unwrap();
        fs::write(store.path(), r#"{"showOsd": false}"#).unwrap();
        let cfg = store.load();
        assert!(!cfg.show_osd);
        assert!(cfg.autostart);
    }

    #[test]
    fn corrupt_file_is_moved_aside_and_defaults_are_used() {
        let (_dir, store) = store();
        fs::create_dir_all(store.path().parent().unwrap()).unwrap();
        fs::write(store.path(), "{ not json").unwrap();
        assert_eq!(store.load(), Config::default());
        assert!(store.path().with_file_name("config.bad.json").exists());
        assert!(!store.path().exists());
    }

    #[test]
    fn recovers_interrupted_capability_reads() {
        let mut cfg = Config::default();
        let key = MonitorKey("BAD#1".into());
        let m = cfg.monitor_mut(&key);
        m.label = "Crashy 27".into();
        m.caps_read_pending = true;
        assert_eq!(cfg.recover_interrupted_caps_reads(), vec!["Crashy 27".to_string()]);
        let m = cfg.monitor(&key).unwrap();
        assert!(!m.caps_read_pending);
        assert!(!m.read_capabilities);
    }

    #[test]
    fn named_ports_lists_only_named_inputs_in_order() {
        let mut m = MonitorConfig::default();
        m.port_mut(0x1B).name = Some("笔记本".into());
        m.port_mut(0x11).hotkey = Some("Ctrl+Alt+2".into());
        m.port_mut(0x0F).name = Some("主机".into());
        assert_eq!(m.named_ports(), vec![0x0F, 0x1B]);
    }
}
```

- [ ] **Step 3: 运行测试，确认失败**

Run: `cargo test --manifest-path src-tauri/Cargo.toml config`
Expected: FAIL（`save_then_load_round_trips`、`corrupt_file_...`、`recovers_...`、`named_ports_...`、`partial_file_...` 失败）

- [ ] **Step 4: 实现 `recover_interrupted_caps_reads`、`named_ports`、`load`、`save`**

```rust
    // in impl Config
    pub fn recover_interrupted_caps_reads(&mut self) -> Vec<String> {
        let mut labels = Vec::new();
        for m in self.monitors.values_mut() {
            if m.caps_read_pending {
                m.caps_read_pending = false;
                m.read_capabilities = false;
                labels.push(m.label.clone());
            }
        }
        labels
    }
```

```rust
    // in impl MonitorConfig
    pub fn named_ports(&self) -> Vec<u16> {
        let mut values: Vec<u16> = self
            .ports
            .iter()
            .filter(|(_, p)| p.name.is_some())
            .filter_map(|(k, _)| k.parse().ok())
            .collect();
        values.sort();
        values
    }
```

```rust
    // in impl ConfigStore
    /// Missing file → defaults. Corrupt file → moved to `config.bad.json`, defaults returned.
    pub fn load(&self) -> Config {
        let Ok(text) = fs::read_to_string(&self.path) else {
            return Config::default();
        };
        match serde_json::from_str::<Config>(&text) {
            Ok(cfg) => cfg,
            Err(_) => {
                let _ = fs::rename(&self.path, self.path.with_file_name("config.bad.json"));
                Config::default()
            }
        }
    }

    /// Write a temporary file, then rename it over the real one, so a crash never leaves half a file.
    pub fn save(&self, cfg: &Config) -> io::Result<()> {
        if let Some(dir) = self.path.parent() {
            fs::create_dir_all(dir)?;
        }
        let tmp = self.path.with_extension("json.tmp");
        let text = serde_json::to_string_pretty(cfg).map_err(io::Error::other)?;
        fs::write(&tmp, text)?;
        fs::rename(&tmp, &self.path)
    }
```

- [ ] **Step 5: 运行测试，确认通过**

Run: `cargo test --manifest-path src-tauri/Cargo.toml config`
Expected: `test result: ok. 7 passed`

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/config.rs src-tauri/src/lib.rs
git commit -m "feat: settings file with atomic save and crash recovery"
```

---

### Task 6: 显示器后端接口和测试用假后端

**Files:**
- Create: `src-tauri/src/ddc/backend.rs`
- Create: `src-tauri/src/ddc/fake.rs`
- Modify: `src-tauri/src/ddc/mod.rs`

- [ ] **Step 1: 写 `ddc/backend.rs`**

```rust
use crate::model::{MonitorInfo, MonitorKey};

/// Everything the app needs from a platform's DDC/CI implementation.
/// All calls happen on the MonitorService worker thread.
pub trait DisplayBackend: Send {
    /// Enumerate monitors again. Handles from the previous call may be released.
    fn list(&mut self) -> Vec<MonitorInfo>;
    /// Current value of VCP 0x60 (Input Select).
    fn read_input(&mut self, key: &MonitorKey) -> Result<u16, String>;
    fn set_input(&mut self, key: &MonitorKey, value: u16) -> Result<(), String>;
    /// The raw capabilities string.
    fn capabilities(&mut self, key: &MonitorKey) -> Result<String, String>;
}
```

- [ ] **Step 2: 写 `ddc/fake.rs`**

```rust
//! In-memory backend for tests. Tests keep a clone and inspect or change the shared state.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use super::backend::DisplayBackend;
use crate::model::{ConnectionKind, MonitorInfo, MonitorKey};

#[derive(Default)]
pub struct FakeState {
    pub monitors: Vec<MonitorInfo>,
    pub inputs: HashMap<MonitorKey, u16>,
    pub caps: HashMap<MonitorKey, String>,
    pub caps_reads: u32,
    pub set_log: Vec<(MonitorKey, u16)>,
    /// The next N set_input calls fail.
    pub fail_next_sets: u32,
    /// set_input succeeds but the input does not change.
    pub ignore_sets: bool,
    pub fail_reads: bool,
}

#[derive(Clone, Default)]
pub struct FakeBackend(pub Arc<Mutex<FakeState>>);

impl DisplayBackend for FakeBackend {
    fn list(&mut self) -> Vec<MonitorInfo> {
        self.0.lock().unwrap().monitors.clone()
    }

    fn read_input(&mut self, key: &MonitorKey) -> Result<u16, String> {
        let s = self.0.lock().unwrap();
        if s.fail_reads {
            return Err("read failed".into());
        }
        s.inputs.get(key).copied().ok_or_else(|| "no such monitor".into())
    }

    fn set_input(&mut self, key: &MonitorKey, value: u16) -> Result<(), String> {
        let mut s = self.0.lock().unwrap();
        if s.fail_next_sets > 0 {
            s.fail_next_sets -= 1;
            return Err("set failed".into());
        }
        s.set_log.push((key.clone(), value));
        if !s.ignore_sets {
            s.inputs.insert(key.clone(), value);
        }
        Ok(())
    }

    fn capabilities(&mut self, key: &MonitorKey) -> Result<String, String> {
        let mut s = self.0.lock().unwrap();
        s.caps_reads += 1;
        s.caps.get(key).cloned().ok_or_else(|| "no capabilities".into())
    }
}

pub fn dell() -> MonitorInfo {
    MonitorInfo {
        key: MonitorKey("DEL426A#UID1".into()),
        label: "Dell U2723QE".into(),
        connection: ConnectionKind::DisplayPort,
    }
}
```

- [ ] **Step 3: 更新 `ddc/mod.rs`**

```rust
pub mod backend;
pub mod caps;
#[cfg(test)]
pub mod fake;
```

- [ ] **Step 4: 确认能编译**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --no-run`
Expected: `Finished`，没有错误

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/ddc
git commit -m "feat: DisplayBackend trait and in-memory fake"
```

---

### Task 7: MonitorService 工作线程

**Files:**
- Create: `src-tauri/src/monitor_service.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: 在 `lib.rs` 加 `pub mod monitor_service;`**

- [ ] **Step 2: 写类型、空的工作线程和会失败的测试**

`src-tauri/src/monitor_service.rs`：

```rust
//! Owns the DDC backend on one worker thread so monitor commands never overlap (spec 5.1, 5.2).

use std::collections::HashMap;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use serde::Serialize;

use crate::ddc::backend::DisplayBackend;
use crate::ddc::caps;
use crate::local_port::guess_local_input;
use crate::model::{ConnectionKind, MonitorInfo, MonitorKey};

pub const NO_RESPONSE: &str = "显示器没响应，检查显示器菜单里的 DDC/CI 是否开启";
pub const NOT_APPLIED: &str = "切换可能没有生效";

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MonitorState {
    pub key: MonitorKey,
    pub label: String,
    pub connection: ConnectionKind,
    pub inputs: Vec<u16>,
    pub current: Option<u16>,
    pub local_input: Option<u16>,
    pub error: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ChangeCause {
    /// This app switched it.
    User,
    /// Changed elsewhere: the monitor's buttons, another computer, auto input select.
    External,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ServiceEvent {
    Snapshot(Vec<MonitorState>),
    InputChanged {
        key: MonitorKey,
        from: Option<u16>,
        to: u16,
        cause: ChangeCause,
        local_input: Option<u16>,
    },
    SwitchFailed {
        key: MonitorKey,
        value: u16,
        reason: String,
    },
}

pub enum CapsDecision {
    Cached(String),
    Read,
    Skip,
}

/// Settings the service needs. Implemented over the config file by the app.
pub trait MonitorPrefs: Send + Sync {
    fn caps_decision(&self, key: &MonitorKey) -> CapsDecision;
    /// Called right before reading capabilities; must be persisted immediately.
    fn begin_caps_read(&self, key: &MonitorKey, label: &str);
    fn finish_caps_read(&self, key: &MonitorKey, raw: Option<String>);
    fn local_override(&self, key: &MonitorKey) -> Option<u16>;
}

#[derive(Clone, Copy, Debug)]
pub struct Timing {
    pub poll: Duration,
    pub verify_delay: Duration,
    pub retry_delay: Duration,
    pub attempts: u32,
    /// Enumerate monitors again every N polls, to notice plugged / unplugged monitors.
    pub relist_every: u32,
    pub poll_failures_before_error: u32,
}

impl Timing {
    pub fn production() -> Self {
        Timing {
            poll: Duration::from_secs(4),
            verify_delay: Duration::from_millis(1500),
            retry_delay: Duration::from_millis(50),
            attempts: 3,
            relist_every: 5,
            poll_failures_before_error: 3,
        }
    }
}

enum Command {
    Refresh,
    Switch { key: MonitorKey, value: u16 },
    Shutdown,
}

pub struct MonitorService {
    tx: Sender<Command>,
    snapshot: Arc<Mutex<Vec<MonitorState>>>,
    handle: Option<thread::JoinHandle<()>>,
}

impl MonitorService {
    pub fn start<F>(
        backend: Box<dyn DisplayBackend>,
        prefs: Arc<dyn MonitorPrefs>,
        is_laptop: bool,
        timing: Timing,
        on_event: F,
    ) -> Self
    where
        F: Fn(ServiceEvent) + Send + 'static,
    {
        let (tx, rx) = mpsc::channel();
        let snapshot = Arc::new(Mutex::new(Vec::new()));
        let mut worker = Worker {
            backend,
            prefs,
            is_laptop,
            timing,
            on_event: Box::new(on_event),
            snapshot: snapshot.clone(),
            monitors: Vec::new(),
            poll_failures: HashMap::new(),
            polls_since_list: 0,
        };
        let handle = thread::spawn(move || worker.run(rx));
        MonitorService { tx, snapshot, handle: Some(handle) }
    }

    /// Enumerate monitors and read everything again.
    pub fn refresh(&self) {
        let _ = self.tx.send(Command::Refresh);
    }

    pub fn switch(&self, key: MonitorKey, value: u16) {
        let _ = self.tx.send(Command::Switch { key, value });
    }

    pub fn snapshot(&self) -> Vec<MonitorState> {
        self.snapshot.lock().unwrap().clone()
    }
}

impl Drop for MonitorService {
    fn drop(&mut self) {
        let _ = self.tx.send(Command::Shutdown);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

struct Worker {
    backend: Box<dyn DisplayBackend>,
    prefs: Arc<dyn MonitorPrefs>,
    is_laptop: bool,
    timing: Timing,
    on_event: Box<dyn Fn(ServiceEvent) + Send>,
    snapshot: Arc<Mutex<Vec<MonitorState>>>,
    monitors: Vec<MonitorState>,
    poll_failures: HashMap<MonitorKey, u32>,
    polls_since_list: u32,
}

impl Worker {
    fn run(&mut self, rx: Receiver<Command>) {
        while let Ok(command) = rx.recv() {
            if let Command::Shutdown = command {
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ddc::caps::DELL_STYLE_CAPS;
    use crate::ddc::fake::{dell, FakeBackend};
    use std::time::Instant;

    fn timing() -> Timing {
        Timing {
            poll: Duration::from_millis(30),
            verify_delay: Duration::from_millis(10),
            retry_delay: Duration::from_millis(1),
            attempts: 3,
            relist_every: 1000,
            poll_failures_before_error: 2,
        }
    }

    #[derive(Default)]
    struct TestPrefs {
        cached: Option<String>,
        override_input: Option<u16>,
        log: Mutex<Vec<String>>,
    }

    impl MonitorPrefs for TestPrefs {
        fn caps_decision(&self, _key: &MonitorKey) -> CapsDecision {
            match &self.cached {
                Some(raw) => CapsDecision::Cached(raw.clone()),
                None => CapsDecision::Read,
            }
        }
        fn begin_caps_read(&self, key: &MonitorKey, _label: &str) {
            self.log.lock().unwrap().push(format!("begin {}", key.0));
        }
        fn finish_caps_read(&self, key: &MonitorKey, raw: Option<String>) {
            self.log.lock().unwrap().push(format!("finish {} {}", key.0, raw.is_some()));
        }
        fn local_override(&self, _key: &MonitorKey) -> Option<u16> {
            self.override_input
        }
    }

    fn fake_with_dell(current: u16) -> FakeBackend {
        let fake = FakeBackend::default();
        {
            let mut s = fake.0.lock().unwrap();
            s.monitors.push(dell());
            s.inputs.insert(dell().key, current);
            s.caps.insert(dell().key, DELL_STYLE_CAPS.to_string());
        }
        fake
    }

    fn start(fake: &FakeBackend, prefs: TestPrefs) -> (MonitorService, Receiver<ServiceEvent>, Arc<TestPrefs>) {
        let prefs = Arc::new(prefs);
        let (tx, rx) = mpsc::channel();
        let service = MonitorService::start(Box::new(fake.clone()), prefs.clone(), false, timing(), move |e| {
            let _ = tx.send(e);
        });
        (service, rx, prefs)
    }

    fn wait_for(rx: &Receiver<ServiceEvent>, pred: impl Fn(&ServiceEvent) -> bool) -> ServiceEvent {
        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            match rx.recv_timeout(left) {
                Ok(e) if pred(&e) => return e,
                Ok(_) => continue,
                Err(_) => panic!("timed out waiting for an event"),
            }
        }
    }

    fn first_snapshot(rx: &Receiver<ServiceEvent>) -> Vec<MonitorState> {
        match wait_for(rx, |e| matches!(e, ServiceEvent::Snapshot(_))) {
            ServiceEvent::Snapshot(states) => states,
            _ => unreachable!(),
        }
    }

    #[test]
    fn first_snapshot_describes_the_monitor() {
        let fake = fake_with_dell(0x0F);
        let (_svc, rx, _) = start(&fake, TestPrefs::default());
        let states = first_snapshot(&rx);
        assert_eq!(states.len(), 1);
        assert_eq!(states[0].current, Some(0x0F));
        assert_eq!(states[0].inputs, vec![0x0F, 0x11, 0x1B]);
        assert_eq!(states[0].local_input, Some(0x0F));
        assert_eq!(states[0].error, None);
    }

    #[test]
    fn reads_capabilities_and_reports_progress() {
        let fake = fake_with_dell(0x0F);
        let (_svc, rx, prefs) = start(&fake, TestPrefs::default());
        first_snapshot(&rx);
        assert_eq!(fake.0.lock().unwrap().caps_reads, 1);
        assert_eq!(
            *prefs.log.lock().unwrap(),
            vec!["begin DEL426A#UID1".to_string(), "finish DEL426A#UID1 true".to_string()]
        );
    }

    #[test]
    fn uses_cached_capabilities_without_asking_the_monitor() {
        let fake = fake_with_dell(0x0F);
        let prefs = TestPrefs { cached: Some(DELL_STYLE_CAPS.to_string()), ..Default::default() };
        let (_svc, rx, _) = start(&fake, prefs);
        assert_eq!(first_snapshot(&rx)[0].inputs, vec![0x0F, 0x11, 0x1B]);
        assert_eq!(fake.0.lock().unwrap().caps_reads, 0);
    }

    #[test]
    fn local_override_wins_over_the_guess() {
        let fake = fake_with_dell(0x0F);
        let prefs = TestPrefs { override_input: Some(0x11), ..Default::default() };
        let (_svc, rx, _) = start(&fake, prefs);
        assert_eq!(first_snapshot(&rx)[0].local_input, Some(0x11));
    }

    #[test]
    fn switch_changes_input_and_reports_a_user_change() {
        let fake = fake_with_dell(0x0F);
        let (svc, rx, _) = start(&fake, TestPrefs::default());
        first_snapshot(&rx);
        svc.switch(dell().key, 0x1B);
        let event = wait_for(&rx, |e| matches!(e, ServiceEvent::InputChanged { .. }));
        assert_eq!(
            event,
            ServiceEvent::InputChanged {
                key: dell().key,
                from: Some(0x0F),
                to: 0x1B,
                cause: ChangeCause::User,
                local_input: Some(0x0F),
            }
        );
        assert_eq!(fake.0.lock().unwrap().set_log, vec![(dell().key, 0x1B)]);
        assert_eq!(svc.snapshot()[0].current, Some(0x1B));
    }

    #[test]
    fn switch_retries_transient_failures() {
        let fake = fake_with_dell(0x0F);
        let (svc, rx, _) = start(&fake, TestPrefs::default());
        first_snapshot(&rx);
        fake.0.lock().unwrap().fail_next_sets = 2;
        svc.switch(dell().key, 0x1B);
        wait_for(&rx, |e| matches!(e, ServiceEvent::InputChanged { cause: ChangeCause::User, .. }));
        assert_eq!(fake.0.lock().unwrap().set_log.len(), 1);
    }

    #[test]
    fn switch_that_never_goes_through_is_reported() {
        let fake = fake_with_dell(0x0F);
        let (svc, rx, _) = start(&fake, TestPrefs::default());
        first_snapshot(&rx);
        fake.0.lock().unwrap().fail_next_sets = 3;
        svc.switch(dell().key, 0x1B);
        let event = wait_for(&rx, |e| matches!(e, ServiceEvent::SwitchFailed { .. }));
        assert_eq!(event, ServiceEvent::SwitchFailed { key: dell().key, value: 0x1B, reason: NO_RESPONSE.into() });
    }

    #[test]
    fn switch_the_monitor_ignores_is_reported() {
        let fake = fake_with_dell(0x0F);
        let (svc, rx, _) = start(&fake, TestPrefs::default());
        first_snapshot(&rx);
        fake.0.lock().unwrap().ignore_sets = true;
        svc.switch(dell().key, 0x1B);
        let event = wait_for(&rx, |e| matches!(e, ServiceEvent::SwitchFailed { .. }));
        assert_eq!(event, ServiceEvent::SwitchFailed { key: dell().key, value: 0x1B, reason: NOT_APPLIED.into() });
    }

    #[test]
    fn polling_notices_changes_made_elsewhere() {
        let fake = fake_with_dell(0x0F);
        let (_svc, rx, _) = start(&fake, TestPrefs::default());
        first_snapshot(&rx);
        fake.0.lock().unwrap().inputs.insert(dell().key, 0x1B);
        let event = wait_for(&rx, |e| matches!(e, ServiceEvent::InputChanged { .. }));
        assert_eq!(
            event,
            ServiceEvent::InputChanged {
                key: dell().key,
                from: Some(0x0F),
                to: 0x1B,
                cause: ChangeCause::External,
                local_input: Some(0x0F),
            }
        );
    }

    #[test]
    fn repeated_read_failures_mark_the_monitor_unresponsive() {
        let fake = fake_with_dell(0x0F);
        let (_svc, rx, _) = start(&fake, TestPrefs::default());
        first_snapshot(&rx);
        fake.0.lock().unwrap().fail_reads = true;
        wait_for(&rx, |e| match e {
            ServiceEvent::Snapshot(states) => states[0].error.as_deref() == Some(NO_RESPONSE),
            _ => false,
        });
    }
}
```

- [ ] **Step 3: 运行测试，确认失败**

Run: `cargo test --manifest-path src-tauri/Cargo.toml monitor_service`
Expected: FAIL，各测试报 "timed out waiting for an event"

- [ ] **Step 4: 实现工作线程**

把 `impl Worker { ... }` 整块替换为：

```rust
impl Worker {
    fn run(&mut self, rx: Receiver<Command>) {
        self.relist();
        loop {
            match rx.recv_timeout(self.timing.poll) {
                Ok(Command::Refresh) => self.relist(),
                Ok(Command::Switch { key, value }) => self.switch(&key, value),
                Ok(Command::Shutdown) | Err(RecvTimeoutError::Disconnected) => break,
                Err(RecvTimeoutError::Timeout) => self.poll(),
            }
        }
    }

    fn relist(&mut self) {
        self.polls_since_list = 0;
        let infos = self.backend.list();
        let mut next = Vec::with_capacity(infos.len());
        for info in infos {
            let inputs = self.inputs_for(&info);
            let previous = self.monitors.iter().find(|m| m.key == info.key).and_then(|m| m.current);
            let (current, error) = match self.read_with_retry(&info.key) {
                Ok(v) => (Some(v), None),
                Err(_) => (previous, Some(NO_RESPONSE.to_string())),
            };
            let local_input = self
                .prefs
                .local_override(&info.key)
                .or_else(|| guess_local_input(info.connection, self.is_laptop, &inputs));
            next.push(MonitorState {
                key: info.key,
                label: info.label,
                connection: info.connection,
                inputs,
                current,
                local_input,
                error,
            });
        }
        self.monitors = next;
        self.publish();
    }

    fn inputs_for(&mut self, info: &MonitorInfo) -> Vec<u16> {
        let raw = match self.prefs.caps_decision(&info.key) {
            CapsDecision::Cached(raw) => Some(raw),
            CapsDecision::Skip => None,
            CapsDecision::Read => {
                self.prefs.begin_caps_read(&info.key, &info.label);
                let raw = self.backend.capabilities(&info.key).ok();
                self.prefs.finish_caps_read(&info.key, raw.clone());
                raw
            }
        };
        raw.map(|r| caps::parse(&r).inputs).unwrap_or_default()
    }

    fn read_once(&mut self, key: &MonitorKey) -> Result<u16, String> {
        // Some monitors pad the value with extra bytes; only the low byte is the input.
        self.backend.read_input(key).map(|v| v & 0x00FF)
    }

    fn read_with_retry(&mut self, key: &MonitorKey) -> Result<u16, String> {
        let mut last = Err(String::new());
        for attempt in 0..self.timing.attempts {
            last = self.read_once(key);
            if last.is_ok() {
                break;
            }
            if attempt + 1 < self.timing.attempts {
                thread::sleep(self.timing.retry_delay);
            }
        }
        last
    }

    fn poll(&mut self) {
        self.polls_since_list += 1;
        if self.monitors.is_empty() || self.polls_since_list >= self.timing.relist_every {
            self.relist();
            return;
        }
        let mut changed = false;
        let mut events = Vec::new();
        for idx in 0..self.monitors.len() {
            let key = self.monitors[idx].key.clone();
            match self.read_once(&key) {
                Ok(v) => {
                    self.poll_failures.remove(&key);
                    let m = &mut self.monitors[idx];
                    if m.error.take().is_some() {
                        changed = true;
                    }
                    if m.current != Some(v) {
                        if m.current.is_some() {
                            events.push(ServiceEvent::InputChanged {
                                key: key.clone(),
                                from: m.current,
                                to: v,
                                cause: ChangeCause::External,
                                local_input: m.local_input,
                            });
                        }
                        m.current = Some(v);
                        changed = true;
                    }
                }
                Err(_) => {
                    let failures = self.poll_failures.entry(key).or_insert(0);
                    *failures += 1;
                    if *failures >= self.timing.poll_failures_before_error && self.monitors[idx].error.is_none() {
                        self.monitors[idx].error = Some(NO_RESPONSE.to_string());
                        changed = true;
                    }
                }
            }
        }
        if changed {
            self.publish();
        }
        for event in events {
            (self.on_event)(event);
        }
    }

    fn switch(&mut self, key: &MonitorKey, value: u16) {
        let mut sent = false;
        for attempt in 0..self.timing.attempts {
            if self.backend.set_input(key, value).is_ok() {
                sent = true;
                break;
            }
            if attempt + 1 < self.timing.attempts {
                thread::sleep(self.timing.retry_delay);
            }
        }
        if !sent {
            (self.on_event)(ServiceEvent::SwitchFailed { key: key.clone(), value, reason: NO_RESPONSE.to_string() });
            return;
        }
        thread::sleep(self.timing.verify_delay);
        let read = self.read_with_retry(key);
        let Some(idx) = self.monitors.iter().position(|m| &m.key == key) else {
            return;
        };
        let from = self.monitors[idx].current;
        let local_input = self.monitors[idx].local_input;
        let event = match read {
            Ok(v) if v != value => {
                self.monitors[idx].current = Some(v);
                ServiceEvent::SwitchFailed { key: key.clone(), value, reason: NOT_APPLIED.to_string() }
            }
            // The expected value, or no answer: many monitors go quiet for a moment while they
            // lock on to the new signal. The command was accepted, so treat it as done.
            _ => {
                self.monitors[idx].current = Some(value);
                self.monitors[idx].error = None;
                ServiceEvent::InputChanged { key: key.clone(), from, to: value, cause: ChangeCause::User, local_input }
            }
        };
        self.publish();
        (self.on_event)(event);
    }

    /// Store the state for `snapshot()` first, then notify, so listeners always read fresh state.
    fn publish(&mut self) {
        *self.snapshot.lock().unwrap() = self.monitors.clone();
        (self.on_event)(ServiceEvent::Snapshot(self.monitors.clone()));
    }
}
```

- [ ] **Step 5: 运行测试，确认通过**

Run: `cargo test --manifest-path src-tauri/Cargo.toml monitor_service`
Expected: `test result: ok. 10 passed`

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/monitor_service.rs src-tauri/src/lib.rs
git commit -m "feat: MonitorService worker with switch verification and polling"
```

---

### Task 8: Windows 显示器后端

**Files:**
- Create: `src-tauri/src/platform/mod.rs`
- Create: `src-tauri/src/platform/stub.rs`
- Create: `src-tauri/src/platform/win/mod.rs`
- Create: `src-tauri/src/platform/win/ffi.rs`
- Create: `src-tauri/src/platform/win/topology.rs`
- Create: `src-tauri/src/platform/win/backend.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: 在 `lib.rs` 加 `pub mod platform;`**

- [ ] **Step 2: 写 `platform/mod.rs` 和 `platform/stub.rs`**

`platform/mod.rs`：

```rust
#[cfg(windows)]
mod win;
#[cfg(windows)]
pub use win::{is_laptop, WinBackend as NativeBackend};

#[cfg(not(windows))]
mod stub;
#[cfg(not(windows))]
pub use stub::{is_laptop, StubBackend as NativeBackend};

/// Name shown for this computer when its input has no name yet.
pub fn computer_name() -> String {
    std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "本机".into())
}
```

`platform/stub.rs`（计划 4 替换为 macOS / Linux 实现）：

```rust
use crate::ddc::backend::DisplayBackend;
use crate::model::{MonitorInfo, MonitorKey};

const UNSUPPORTED: &str = "this platform is not supported yet";

#[derive(Default)]
pub struct StubBackend;

impl StubBackend {
    pub fn new() -> Self {
        StubBackend
    }
}

impl DisplayBackend for StubBackend {
    fn list(&mut self) -> Vec<MonitorInfo> {
        Vec::new()
    }
    fn read_input(&mut self, _key: &MonitorKey) -> Result<u16, String> {
        Err(UNSUPPORTED.into())
    }
    fn set_input(&mut self, _key: &MonitorKey, _value: u16) -> Result<(), String> {
        Err(UNSUPPORTED.into())
    }
    fn capabilities(&mut self, _key: &MonitorKey) -> Result<String, String> {
        Err(UNSUPPORTED.into())
    }
}

pub fn is_laptop() -> bool {
    false
}
```

- [ ] **Step 3: 写 `platform/win/ffi.rs`（Win32 原始声明）**

```rust
//! Raw Win32 declarations. Handles are pointer-sized integers (isize), which keeps them Send.
#![allow(non_snake_case, clippy::upper_case_acronyms)]

#[repr(C)]
#[derive(Clone, Copy)]
pub struct RECT {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct MONITORINFOEXW {
    pub cb_size: u32,
    pub rc_monitor: RECT,
    pub rc_work: RECT,
    pub flags: u32,
    pub sz_device: [u16; 32],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct DISPLAY_DEVICEW {
    pub cb: u32,
    pub device_name: [u16; 32],
    pub device_string: [u16; 128],
    pub state_flags: u32,
    pub device_id: [u16; 128],
    pub device_key: [u16; 128],
}

/// Declared with pack(1) in the SDK; on 32- and 64-bit Windows the layout is the same as repr(C).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct PHYSICAL_MONITOR {
    pub handle: isize,
    pub description: [u16; 128],
}

#[repr(C)]
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub struct LUID {
    pub low: u32,
    pub high: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct DISPLAYCONFIG_RATIONAL {
    pub numerator: u32,
    pub denominator: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct DISPLAYCONFIG_PATH_SOURCE_INFO {
    pub adapter_id: LUID,
    pub id: u32,
    pub mode_info_idx: u32,
    pub status_flags: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct DISPLAYCONFIG_PATH_TARGET_INFO {
    pub adapter_id: LUID,
    pub id: u32,
    pub mode_info_idx: u32,
    pub output_technology: u32,
    pub rotation: u32,
    pub scaling: u32,
    pub refresh_rate: DISPLAYCONFIG_RATIONAL,
    pub scan_line_ordering: u32,
    pub target_available: i32,
    pub status_flags: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct DISPLAYCONFIG_PATH_INFO {
    pub source_info: DISPLAYCONFIG_PATH_SOURCE_INFO,
    pub target_info: DISPLAYCONFIG_PATH_TARGET_INFO,
    pub flags: u32,
}

/// The union payload is never read; only its size (48 bytes, 8-byte aligned) matters.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct DISPLAYCONFIG_MODE_INFO {
    pub info_type: u32,
    pub id: u32,
    pub adapter_id: LUID,
    pub payload: [u64; 6],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct DISPLAYCONFIG_DEVICE_INFO_HEADER {
    pub kind: u32,
    pub size: u32,
    pub adapter_id: LUID,
    pub id: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct DISPLAYCONFIG_TARGET_DEVICE_NAME {
    pub header: DISPLAYCONFIG_DEVICE_INFO_HEADER,
    pub flags: u32,
    pub output_technology: u32,
    pub edid_manufacture_id: u16,
    pub edid_product_code_id: u16,
    pub connector_instance: u32,
    pub monitor_friendly_device_name: [u16; 64],
    pub monitor_device_path: [u16; 128],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SYSTEM_POWER_STATUS {
    pub ac_line_status: u8,
    pub battery_flag: u8,
    pub battery_life_percent: u8,
    pub system_status_flag: u8,
    pub battery_life_time: u32,
    pub battery_full_life_time: u32,
}

pub const EDD_GET_DEVICE_INTERFACE_NAME: u32 = 0x1;
pub const DISPLAY_DEVICE_ACTIVE: u32 = 0x1;
pub const QDC_ONLY_ACTIVE_PATHS: u32 = 0x2;
pub const DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME: u32 = 2;
pub const ERROR_INSUFFICIENT_BUFFER: i32 = 122;

pub type MonitorEnumProc = unsafe extern "system" fn(isize, isize, *mut RECT, isize) -> i32;

#[link(name = "user32")]
extern "system" {
    pub fn EnumDisplayMonitors(hdc: isize, clip: *const RECT, callback: Option<MonitorEnumProc>, data: isize) -> i32;
    pub fn GetMonitorInfoW(monitor: isize, info: *mut MONITORINFOEXW) -> i32;
    pub fn EnumDisplayDevicesW(device: *const u16, index: u32, info: *mut DISPLAY_DEVICEW, flags: u32) -> i32;
    pub fn GetDisplayConfigBufferSizes(flags: u32, path_count: *mut u32, mode_count: *mut u32) -> i32;
    pub fn QueryDisplayConfig(
        flags: u32,
        path_count: *mut u32,
        paths: *mut DISPLAYCONFIG_PATH_INFO,
        mode_count: *mut u32,
        modes: *mut DISPLAYCONFIG_MODE_INFO,
        topology: *mut u32,
    ) -> i32;
    pub fn DisplayConfigGetDeviceInfo(header: *mut DISPLAYCONFIG_DEVICE_INFO_HEADER) -> i32;
}

#[link(name = "dxva2")]
extern "system" {
    pub fn GetNumberOfPhysicalMonitorsFromHMONITOR(monitor: isize, count: *mut u32) -> i32;
    pub fn GetPhysicalMonitorsFromHMONITOR(monitor: isize, count: u32, monitors: *mut PHYSICAL_MONITOR) -> i32;
    pub fn DestroyPhysicalMonitor(handle: isize) -> i32;
    pub fn GetVCPFeatureAndVCPFeatureReply(handle: isize, code: u8, code_type: *mut u32, current: *mut u32, maximum: *mut u32) -> i32;
    pub fn SetVCPFeature(handle: isize, code: u8, value: u32) -> i32;
    pub fn GetCapabilitiesStringLength(handle: isize, length: *mut u32) -> i32;
    pub fn CapabilitiesRequestAndCapabilitiesReply(handle: isize, buffer: *mut u8, length: u32) -> i32;
}

#[link(name = "kernel32")]
extern "system" {
    pub fn GetSystemPowerStatus(status: *mut SYSTEM_POWER_STATUS) -> i32;
}

pub fn wide_to_string(wide: &[u16]) -> String {
    let end = wide.iter().position(|&c| c == 0).unwrap_or(wide.len());
    String::from_utf16_lossy(&wide[..end])
}

pub fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::size_of;

    /// Sizes from the Windows SDK headers. A mismatch means Windows would write past our buffers.
    #[test]
    fn struct_sizes_match_the_sdk() {
        assert_eq!(size_of::<DISPLAYCONFIG_PATH_INFO>(), 72);
        assert_eq!(size_of::<DISPLAYCONFIG_MODE_INFO>(), 64);
        assert_eq!(size_of::<DISPLAYCONFIG_TARGET_DEVICE_NAME>(), 420);
        assert_eq!(size_of::<DISPLAY_DEVICEW>(), 840);
        assert_eq!(size_of::<MONITORINFOEXW>(), 104);
        assert_eq!(size_of::<SYSTEM_POWER_STATUS>(), 12);
    }

    #[test]
    fn converts_wide_strings() {
        assert_eq!(wide_to_string(&[0x44, 0x50, 0, 0x58]), "DP");
        assert_eq!(to_wide("A"), vec![0x41, 0]);
    }
}
```

- [ ] **Step 4: 写 `platform/win/topology.rs`**

```rust
//! Which connector each active monitor uses, from QueryDisplayConfig.

use std::mem::{size_of, zeroed};
use std::ptr::null_mut;

use super::ffi::*;
use crate::model::ConnectionKind;

#[derive(Clone, Debug)]
pub struct Target {
    /// Lower-case monitor interface path, e.g. `\\?\display#del426a#5&33621b4e&0&uid4358#{e6f07b5f-...}`.
    pub device_path: String,
    pub friendly_name: String,
    pub connection: ConnectionKind,
}

/// DISPLAYCONFIG_VIDEO_OUTPUT_TECHNOLOGY → ConnectionKind.
/// 19 is DISPLAYPORT_USB_TUNNEL on recent Windows 11 builds; older SDKs do not define it.
pub fn connection_from_technology(technology: u32) -> ConnectionKind {
    match technology {
        5 => ConnectionKind::Hdmi,
        10 | 11 => ConnectionKind::DisplayPort,
        19 => ConnectionKind::UsbTunnel,
        _ => ConnectionKind::Other,
    }
}

pub fn active_targets() -> Vec<Target> {
    unsafe {
        let mut paths: Vec<DISPLAYCONFIG_PATH_INFO>;
        let mut path_count = 0u32;
        loop {
            let mut mode_count = 0u32;
            if GetDisplayConfigBufferSizes(QDC_ONLY_ACTIVE_PATHS, &mut path_count, &mut mode_count) != 0 {
                return Vec::new();
            }
            paths = vec![zeroed(); path_count as usize];
            let mut modes: Vec<DISPLAYCONFIG_MODE_INFO> = vec![zeroed(); mode_count as usize];
            let rc = QueryDisplayConfig(
                QDC_ONLY_ACTIVE_PATHS,
                &mut path_count,
                paths.as_mut_ptr(),
                &mut mode_count,
                modes.as_mut_ptr(),
                null_mut(),
            );
            if rc == ERROR_INSUFFICIENT_BUFFER {
                continue; // the display setup changed between the two calls
            }
            if rc != 0 {
                return Vec::new();
            }
            break;
        }
        paths.truncate(path_count as usize);
        paths
            .iter()
            .filter_map(|p| target_name(p.target_info.adapter_id, p.target_info.id))
            .collect()
    }
}

unsafe fn target_name(adapter_id: LUID, id: u32) -> Option<Target> {
    let mut name: DISPLAYCONFIG_TARGET_DEVICE_NAME = zeroed();
    name.header.kind = DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME;
    name.header.size = size_of::<DISPLAYCONFIG_TARGET_DEVICE_NAME>() as u32;
    name.header.adapter_id = adapter_id;
    name.header.id = id;
    if DisplayConfigGetDeviceInfo(&mut name.header) != 0 {
        return None;
    }
    Some(Target {
        device_path: wide_to_string(&name.monitor_device_path).to_ascii_lowercase(),
        friendly_name: wide_to_string(&name.monitor_friendly_device_name),
        connection: connection_from_technology(name.output_technology),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_output_technologies() {
        assert_eq!(connection_from_technology(5), ConnectionKind::Hdmi);
        assert_eq!(connection_from_technology(10), ConnectionKind::DisplayPort);
        assert_eq!(connection_from_technology(11), ConnectionKind::DisplayPort);
        assert_eq!(connection_from_technology(19), ConnectionKind::UsbTunnel);
        assert_eq!(connection_from_technology(0x8000_0000), ConnectionKind::Other);
    }
}
```

- [ ] **Step 5: 写 `platform/win/backend.rs`（先写纯函数的测试）**

```rust
//! DDC/CI over dxva2, matched to connectors from QueryDisplayConfig by monitor device path.

use std::mem::{size_of, zeroed};
use std::ptr::{null, null_mut};

use super::ffi::*;
use super::topology;
use crate::ddc::backend::DisplayBackend;
use crate::ddc::caps::VCP_INPUT_SELECT;
use crate::model::{ConnectionKind, MonitorInfo, MonitorKey};

/// `\\?\display#del426a#5&33621b4e&0&uid4358#{guid}` → `DEL426A#5&33621B4E&0&UID4358`.
pub fn key_from_device_path(path: &str) -> Option<String> {
    let parts: Vec<&str> = path.split('#').collect();
    if parts.len() < 3 || parts[1].is_empty() || parts[2].is_empty() {
        return None;
    }
    Some(format!("{}#{}", parts[1], parts[2]).to_ascii_uppercase())
}

/// The driver's description ("Dell U2723QE (DP HDR)") without the connection suffix, or the
/// EDID name when the description is the generic one.
pub fn label_for(description: &str, friendly_name: Option<&str>) -> String {
    let description = description.trim();
    let generic = description.is_empty()
        || description.to_ascii_lowercase().starts_with("generic")
        || description.contains("通用");
    let friendly = friendly_name.map(str::trim).filter(|f| !f.is_empty());
    if generic {
        if let Some(f) = friendly {
            return f.to_string();
        }
    }
    let trimmed = match description.rfind(" (") {
        Some(i) if description.ends_with(')') => &description[..i],
        _ => description,
    };
    if trimmed.is_empty() {
        friendly.unwrap_or("显示器").to_string()
    } else {
        trimmed.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_keys_from_device_paths() {
        assert_eq!(
            key_from_device_path(r"\\?\display#del426a#5&33621b4e&0&uid4358#{e6f07b5f-ee97-4a90-b076-33f57bf4eaa7}"),
            Some("DEL426A#5&33621B4E&0&UID4358".to_string())
        );
        assert_eq!(key_from_device_path("garbage"), None);
    }

    #[test]
    fn labels_monitors() {
        assert_eq!(label_for("Dell U2723QE (DP HDR)", Some("DELL U2723QE")), "Dell U2723QE");
        assert_eq!(label_for("Generic PnP Monitor", Some("LG ULTRAFINE")), "LG ULTRAFINE");
        assert_eq!(label_for("通用即插即用监视器", Some("Mi TV")), "Mi TV");
        assert_eq!(label_for("", None), "显示器");
    }
}
```

Run: `cargo test --manifest-path src-tauri/Cargo.toml platform`
Expected: 编译失败，因为 `platform/mod.rs` 引用的 `WinBackend`、`is_laptop` 还不存在。这一步就是确认测试还没有对应实现。

- [ ] **Step 6: 实现 `WinBackend` 和 `is_laptop`**

在 `platform/win/backend.rs` 的 `#[cfg(test)]` 之前加入：

```rust
struct Physical {
    info: MonitorInfo,
    handle: isize,
}

pub struct WinBackend {
    monitors: Vec<Physical>,
}

impl WinBackend {
    pub fn new() -> Self {
        WinBackend { monitors: Vec::new() }
    }

    fn release(&mut self) {
        for m in self.monitors.drain(..) {
            unsafe {
                DestroyPhysicalMonitor(m.handle);
            }
        }
    }

    fn handle(&self, key: &MonitorKey) -> Result<isize, String> {
        self.monitors
            .iter()
            .find(|m| &m.info.key == key)
            .map(|m| m.handle)
            .ok_or_else(|| "monitor not found".to_string())
    }
}

impl Default for WinBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for WinBackend {
    fn drop(&mut self) {
        self.release();
    }
}

impl DisplayBackend for WinBackend {
    fn list(&mut self) -> Vec<MonitorInfo> {
        self.release();
        let targets = topology::active_targets();
        for hmonitor in enum_monitors() {
            let device = monitor_device_name(hmonitor);
            let paths = active_monitor_paths(&device);
            // The i-th physical monitor of an HMONITOR is the i-th active monitor device on it.
            for (i, (handle, description)) in physical_monitors(hmonitor).into_iter().enumerate() {
                let path = paths.get(i);
                let target = path.and_then(|p| targets.iter().find(|t| &t.device_path == p));
                let key = path
                    .and_then(|p| key_from_device_path(p))
                    .unwrap_or_else(|| format!("{device}#{i}"));
                let info = MonitorInfo {
                    key: MonitorKey(key),
                    label: label_for(&description, target.map(|t| t.friendly_name.as_str())),
                    connection: target.map(|t| t.connection).unwrap_or(ConnectionKind::Other),
                };
                self.monitors.push(Physical { info, handle });
            }
        }
        self.monitors.iter().map(|m| m.info.clone()).collect()
    }

    fn read_input(&mut self, key: &MonitorKey) -> Result<u16, String> {
        let handle = self.handle(key)?;
        let (mut current, mut maximum) = (0u32, 0u32);
        let ok = unsafe { GetVCPFeatureAndVCPFeatureReply(handle, VCP_INPUT_SELECT, null_mut(), &mut current, &mut maximum) };
        if ok == 0 {
            return Err("DDC/CI read failed".into());
        }
        Ok((current & 0xFFFF) as u16)
    }

    fn set_input(&mut self, key: &MonitorKey, value: u16) -> Result<(), String> {
        let handle = self.handle(key)?;
        if unsafe { SetVCPFeature(handle, VCP_INPUT_SELECT, value as u32) } == 0 {
            return Err("DDC/CI write failed".into());
        }
        Ok(())
    }

    fn capabilities(&mut self, key: &MonitorKey) -> Result<String, String> {
        let handle = self.handle(key)?;
        let mut length = 0u32;
        if unsafe { GetCapabilitiesStringLength(handle, &mut length) } == 0 || length == 0 {
            return Err("capabilities length unavailable".into());
        }
        let mut buffer = vec![0u8; length as usize];
        if unsafe { CapabilitiesRequestAndCapabilitiesReply(handle, buffer.as_mut_ptr(), length) } == 0 {
            return Err("capabilities request failed".into());
        }
        let end = buffer.iter().position(|&b| b == 0).unwrap_or(buffer.len());
        Ok(String::from_utf8_lossy(&buffer[..end]).into_owned())
    }
}

unsafe extern "system" fn collect_monitor(hmonitor: isize, _hdc: isize, _rect: *mut RECT, data: isize) -> i32 {
    let list = &mut *(data as *mut Vec<isize>);
    list.push(hmonitor);
    1
}

fn enum_monitors() -> Vec<isize> {
    let mut list: Vec<isize> = Vec::new();
    unsafe {
        EnumDisplayMonitors(0, null(), Some(collect_monitor), &mut list as *mut Vec<isize> as isize);
    }
    list
}

fn monitor_device_name(hmonitor: isize) -> String {
    let mut info: MONITORINFOEXW = unsafe { zeroed() };
    info.cb_size = size_of::<MONITORINFOEXW>() as u32;
    if unsafe { GetMonitorInfoW(hmonitor, &mut info) } == 0 {
        return String::new();
    }
    wide_to_string(&info.sz_device)
}

/// Interface paths of the monitor devices that are actually showing this desktop (not just attached).
fn active_monitor_paths(device: &str) -> Vec<String> {
    let device = to_wide(device);
    let mut paths = Vec::new();
    for index in 0u32.. {
        let mut dd: DISPLAY_DEVICEW = unsafe { zeroed() };
        dd.cb = size_of::<DISPLAY_DEVICEW>() as u32;
        if unsafe { EnumDisplayDevicesW(device.as_ptr(), index, &mut dd, EDD_GET_DEVICE_INTERFACE_NAME) } == 0 {
            break;
        }
        if dd.state_flags & DISPLAY_DEVICE_ACTIVE != 0 {
            paths.push(wide_to_string(&dd.device_id).to_ascii_lowercase());
        }
    }
    paths
}

fn physical_monitors(hmonitor: isize) -> Vec<(isize, String)> {
    let mut count = 0u32;
    if unsafe { GetNumberOfPhysicalMonitorsFromHMONITOR(hmonitor, &mut count) } == 0 || count == 0 {
        return Vec::new();
    }
    let mut list: Vec<PHYSICAL_MONITOR> = vec![unsafe { zeroed() }; count as usize];
    if unsafe { GetPhysicalMonitorsFromHMONITOR(hmonitor, count, list.as_mut_ptr()) } == 0 {
        return Vec::new();
    }
    list.iter().map(|p| (p.handle, wide_to_string(&p.description))).collect()
}

/// A battery means a laptop. 128 = no system battery, 255 = unknown.
pub fn is_laptop() -> bool {
    let mut status: SYSTEM_POWER_STATUS = unsafe { zeroed() };
    if unsafe { GetSystemPowerStatus(&mut status) } == 0 {
        return false;
    }
    status.battery_flag != 128 && status.battery_flag != 255
}
```

在 `backend.rs` 的 `mod tests` 里再加一个需要真机的测试（默认不运行）：

```rust
    /// Talks to the real monitors. Run with: cargo test -- --ignored --nocapture lists_real_monitors
    #[test]
    #[ignore]
    fn lists_real_monitors() {
        let mut backend = WinBackend::new();
        let monitors = backend.list();
        for m in &monitors {
            println!("{:?} input={:?}", m, backend.read_input(&m.key));
        }
        assert!(!monitors.is_empty());
    }
```

`platform/win/mod.rs`：

```rust
mod backend;
mod ffi;
mod topology;

pub use backend::{is_laptop, WinBackend};
```

- [ ] **Step 7: 运行测试，确认通过**

Run: `cargo test --manifest-path src-tauri/Cargo.toml platform`
Expected: `test result: ok. 5 passed; 0 failed; 1 ignored`

- [ ] **Step 8: 在真机上运行忽略的测试**

Run: `cargo test --manifest-path src-tauri/Cargo.toml lists_real_monitors -- --ignored --nocapture`
Expected: 打印一行，类似 `MonitorInfo { key: MonitorKey("DEL426A#5&33621B4E&0&UID4358"), label: "Dell U2723QE", connection: DisplayPort } input=Ok(15)`。显卡诱骗器不应出现在列表里（它不是活动显示设备，也不响应 DDC/CI）。如果 `connection` 是 `Other`，说明设备路径没匹配上：打印 `topology::active_targets()` 和 `active_monitor_paths` 对比字符串。

- [ ] **Step 9: Commit**

```bash
git add src-tauri/src/platform src-tauri/src/lib.rs
git commit -m "feat: Windows DDC/CI backend with connector detection"
```

---

### Task 9: 界面数据（View）

**Files:**
- Create: `src-tauri/src/view.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: 在 `lib.rs` 加 `pub mod view;`**

- [ ] **Step 2: 写类型和会失败的测试**

`src-tauri/src/view.rs`：

```rust
//! Everything the UI shows, computed from monitor state and settings.

use serde::Serialize;

use crate::config::Config;
use crate::ddc::caps::input_label;
use crate::model::{ConnectionKind, MonitorKey};
use crate::monitor_service::MonitorState;
use crate::toggle::default_pair;

pub const UNNAMED: &str = "未命名";

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PortView {
    pub value: u16,
    /// "DP", "HDMI", "USB-C"…
    pub port_label: String,
    /// The name the user gave it.
    pub name: Option<String>,
    /// What to show: the name, this computer's name, or "未命名".
    pub display_name: String,
    pub is_local: bool,
    pub is_current: bool,
    pub hotkey: Option<String>,
    /// "desktop" | "laptop" | "plug"
    pub icon: &'static str,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MonitorView {
    pub key: MonitorKey,
    pub label: String,
    pub connection: ConnectionKind,
    pub current: Option<u16>,
    pub current_name: Option<String>,
    pub error: Option<String>,
    pub ports: Vec<PortView>,
    pub toggle_pair: Option<[u16; 2]>,
    pub toggle_pair_is_default: bool,
    pub local_override: Option<u16>,
    pub read_capabilities: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct View {
    pub monitors: Vec<MonitorView>,
    pub show_osd: bool,
    pub autostart: bool,
    pub lan_discovery: bool,
    pub toggle_hotkey: Option<String>,
    pub computer_name: String,
    /// Accelerators the OS refused to register (filled in by AppState).
    pub failed_hotkeys: Vec<String>,
}

pub fn build_view(_states: &[MonitorState], cfg: &Config, _is_laptop: bool, computer_name: &str) -> View {
    View {
        monitors: Vec::new(),
        show_osd: cfg.show_osd,
        autostart: cfg.autostart,
        lan_discovery: cfg.lan_discovery,
        toggle_hotkey: cfg.toggle_hotkey.clone(),
        computer_name: computer_name.to_string(),
        failed_hotkeys: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key() -> MonitorKey {
        MonitorKey("DEL426A#UID1".into())
    }

    fn state(current: Option<u16>, local: Option<u16>) -> MonitorState {
        MonitorState {
            key: key(),
            label: "Dell U2723QE".into(),
            connection: ConnectionKind::DisplayPort,
            inputs: vec![0x0F, 0x11, 0x1B],
            current,
            local_input: local,
            error: None,
        }
    }

    fn values(view: &View) -> Vec<u16> {
        view.monitors[0].ports.iter().map(|p| p.value).collect()
    }

    #[test]
    fn local_port_comes_first_and_shows_the_computer_name() {
        let view = build_view(&[state(Some(0x0F), Some(0x0F))], &Config::default(), false, "DESKTOP-1");
        let port = &view.monitors[0].ports[0];
        assert_eq!(port.value, 0x0F);
        assert!(port.is_local && port.is_current);
        assert_eq!(port.display_name, "DESKTOP-1");
        assert_eq!(port.port_label, "DP");
        assert_eq!(port.icon, "desktop");
        assert_eq!(view.monitors[0].current_name.as_deref(), Some("DESKTOP-1"));
    }

    #[test]
    fn named_ports_come_before_unnamed_ones() {
        let mut cfg = Config::default();
        cfg.monitor_mut(&key()).port_mut(0x1B).name = Some("公司笔记本".into());
        let view = build_view(&[state(Some(0x1B), Some(0x0F))], &cfg, false, "DESKTOP-1");
        assert_eq!(values(&view), vec![0x0F, 0x1B, 0x11]);
        let ports = &view.monitors[0].ports;
        assert_eq!(ports[1].display_name, "公司笔记本");
        assert_eq!(ports[1].icon, "laptop");
        assert!(ports[1].is_current);
        assert_eq!(ports[2].display_name, UNNAMED);
        assert_eq!(ports[2].icon, "plug");
        assert_eq!(view.monitors[0].current_name.as_deref(), Some("公司笔记本"));
    }

    #[test]
    fn configured_ports_the_monitor_does_not_list_are_shown() {
        let mut cfg = Config::default();
        cfg.monitor_mut(&key()).port_mut(0x12).name = Some("游戏机".into());
        let view = build_view(&[state(Some(0x0F), Some(0x0F))], &cfg, false, "PC");
        assert!(values(&view).contains(&0x12));
    }

    #[test]
    fn default_toggle_pair_is_local_plus_first_named() {
        let mut cfg = Config::default();
        cfg.monitor_mut(&key()).port_mut(0x1B).name = Some("笔记本".into());
        let view = build_view(&[state(Some(0x0F), Some(0x0F))], &cfg, false, "PC");
        assert_eq!(view.monitors[0].toggle_pair, Some([0x0F, 0x1B]));
        assert!(view.monitors[0].toggle_pair_is_default);
    }

    #[test]
    fn configured_toggle_pair_wins() {
        let mut cfg = Config::default();
        cfg.monitor_mut(&key()).toggle_pair = Some([0x11, 0x0F]);
        let view = build_view(&[state(Some(0x0F), Some(0x0F))], &cfg, false, "PC");
        assert_eq!(view.monitors[0].toggle_pair, Some([0x11, 0x0F]));
        assert!(!view.monitors[0].toggle_pair_is_default);
    }

    #[test]
    fn on_a_laptop_the_icons_swap() {
        let mut cfg = Config::default();
        cfg.monitor_mut(&key()).port_mut(0x0F).name = Some("主机".into());
        let view = build_view(&[state(Some(0x1B), Some(0x1B))], &cfg, true, "LAPTOP");
        let ports = &view.monitors[0].ports;
        assert_eq!((ports[0].value, ports[0].icon), (0x1B, "laptop"));
        assert_eq!((ports[1].value, ports[1].icon), (0x0F, "desktop"));
    }

    #[test]
    fn copies_options_and_hotkeys() {
        let mut cfg = Config::default();
        cfg.show_osd = false;
        cfg.monitor_mut(&key()).port_mut(0x1B).hotkey = Some("Ctrl+Alt+2".into());
        let view = build_view(&[state(Some(0x0F), Some(0x0F))], &cfg, false, "PC");
        assert!(!view.show_osd);
        assert_eq!(view.toggle_hotkey.as_deref(), Some("Ctrl+Alt+M"));
        let usb_c = view.monitors[0].ports.iter().find(|p| p.value == 0x1B).unwrap();
        assert_eq!(usb_c.hotkey.as_deref(), Some("Ctrl+Alt+2"));
    }
}
```

- [ ] **Step 3: 运行测试，确认失败**

Run: `cargo test --manifest-path src-tauri/Cargo.toml view`
Expected: FAIL（`index out of bounds`，因为 `monitors` 是空的）

- [ ] **Step 4: 实现 `build_view`**

把 `build_view` 替换为：

```rust
pub fn build_view(states: &[MonitorState], cfg: &Config, is_laptop: bool, computer_name: &str) -> View {
    View {
        monitors: states.iter().map(|s| monitor_view(s, cfg, is_laptop, computer_name)).collect(),
        show_osd: cfg.show_osd,
        autostart: cfg.autostart,
        lan_discovery: cfg.lan_discovery,
        toggle_hotkey: cfg.toggle_hotkey.clone(),
        computer_name: computer_name.to_string(),
        failed_hotkeys: Vec::new(),
    }
}

fn monitor_view(s: &MonitorState, cfg: &Config, is_laptop: bool, computer_name: &str) -> MonitorView {
    let mc = cfg.monitor(&s.key);

    let mut values = s.inputs.clone();
    if let Some(mc) = mc {
        values.extend(mc.ports.keys().filter_map(|k| k.parse::<u16>().ok()));
    }
    values.extend(s.current);
    values.extend(s.local_input);
    values.sort();
    values.dedup();

    let mut ports: Vec<PortView> = values
        .into_iter()
        .map(|value| {
            let pc = mc.and_then(|m| m.port(value));
            let name = pc.and_then(|p| p.name.clone());
            let is_local = s.local_input == Some(value);
            let display_name = match (&name, is_local) {
                (Some(n), _) => n.clone(),
                (None, true) => computer_name.to_string(),
                (None, false) => UNNAMED.to_string(),
            };
            // With two computers on one monitor, the other one is usually the other kind.
            let (this_kind, other_kind) = if is_laptop { ("laptop", "desktop") } else { ("desktop", "laptop") };
            let icon = if is_local {
                this_kind
            } else if name.is_some() {
                other_kind
            } else {
                "plug"
            };
            PortView {
                value,
                port_label: input_label(value),
                name,
                display_name,
                is_local,
                is_current: s.current == Some(value),
                hotkey: pc.and_then(|p| p.hotkey.clone()),
                icon,
            }
        })
        .collect();
    ports.sort_by_key(|p| (!p.is_local, p.name.is_none(), p.value));

    let named: Vec<u16> = ports.iter().filter(|p| p.name.is_some()).map(|p| p.value).collect();
    let configured_pair = mc.and_then(|m| m.toggle_pair);
    let toggle_pair = configured_pair.or_else(|| default_pair(s.local_input, &named, &s.inputs));
    let current_name = s
        .current
        .and_then(|c| ports.iter().find(|p| p.value == c))
        .map(|p| p.display_name.clone());

    MonitorView {
        key: s.key.clone(),
        label: s.label.clone(),
        connection: s.connection,
        current: s.current,
        current_name,
        error: s.error.clone(),
        ports,
        toggle_pair,
        toggle_pair_is_default: configured_pair.is_none(),
        local_override: mc.and_then(|m| m.local_port_override),
        read_capabilities: mc.map(|m| m.read_capabilities).unwrap_or(true),
    }
}
```

- [ ] **Step 5: 运行测试，确认通过**

Run: `cargo test --manifest-path src-tauri/Cargo.toml view`
Expected: `test result: ok. 7 passed`

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/view.rs src-tauri/src/lib.rs
git commit -m "feat: build the UI view from monitor state and settings"
```

---

### Task 10: 快捷键绑定和冲突检查

**Files:**
- Create: `src-tauri/src/hotkeys.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: 在 `lib.rs` 加 `pub mod hotkeys;`**

- [ ] **Step 2: 写类型、注册表和会失败的测试**

`src-tauri/src/hotkeys.rs`：

```rust
//! Global hotkeys: which accelerator does what, conflicts, and registration with the OS.

use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Mutex;

use tauri::{AppHandle, Runtime};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

use crate::config::Config;
use crate::model::MonitorKey;

pub const BAD_FORMAT: &str = "这个组合键用不了，换一个";
pub const IN_USE_HERE: &str = "这个快捷键已经给别的接口用了";
pub const IN_USE_ELSEWHERE: &str = "已被其他程序占用，换一个";

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HotkeyAction {
    ToggleAll,
    SwitchTo { key: MonitorKey, value: u16 },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HotkeyTarget {
    Toggle,
    Port { key: MonitorKey, value: u16 },
}

/// The OS-level id of an accelerator such as "Ctrl+Alt+M" (case-insensitive).
pub fn shortcut_id(_accelerator: &str) -> Result<u32, String> {
    Err(BAD_FORMAT.into())
}

/// Every configured hotkey: the toggle first, then per-input hotkeys.
pub fn bindings(_cfg: &Config) -> Vec<(String, HotkeyAction)> {
    Vec::new()
}

pub fn get(cfg: &Config, target: &HotkeyTarget) -> Option<String> {
    match target {
        HotkeyTarget::Toggle => cfg.toggle_hotkey.clone(),
        HotkeyTarget::Port { key, value } => cfg.monitor(key).and_then(|m| m.port(*value)).and_then(|p| p.hotkey.clone()),
    }
}

pub fn set(cfg: &mut Config, target: &HotkeyTarget, accelerator: Option<String>) {
    match target {
        HotkeyTarget::Toggle => cfg.toggle_hotkey = accelerator,
        HotkeyTarget::Port { key, value } => cfg.monitor_mut(key).port_mut(*value).hotkey = accelerator,
    }
}

/// True when `accelerator` is already bound to something other than `target`.
pub fn conflicts(_cfg: &Config, _target: &HotkeyTarget, _accelerator: &str) -> bool {
    false
}

#[derive(Default)]
pub struct HotkeyRegistry {
    actions: Mutex<HashMap<u32, HotkeyAction>>,
}

impl HotkeyRegistry {
    pub fn action_for(&self, id: u32) -> Option<HotkeyAction> {
        self.actions.lock().unwrap().get(&id).cloned()
    }

    /// Replace every registered shortcut with `bindings`.
    /// Returns the accelerators the OS refused (usually taken by another program).
    pub fn apply<R: Runtime>(&self, app: &AppHandle<R>, bindings: Vec<(String, HotkeyAction)>) -> Vec<String> {
        let shortcuts = app.global_shortcut();
        let _ = shortcuts.unregister_all();
        let mut actions = self.actions.lock().unwrap();
        actions.clear();
        let mut failed = Vec::new();
        for (accelerator, action) in bindings {
            match Shortcut::from_str(&accelerator) {
                Ok(shortcut) => {
                    let id = shortcut.id();
                    if shortcuts.register(shortcut).is_ok() {
                        actions.insert(id, action);
                    } else {
                        failed.push(accelerator);
                    }
                }
                Err(_) => failed.push(accelerator),
            }
        }
        failed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key() -> MonitorKey {
        MonitorKey("DEL426A#UID1".into())
    }

    fn config() -> Config {
        let mut cfg = Config::default();
        cfg.monitor_mut(&key()).port_mut(0x1B).hotkey = Some("Ctrl+Alt+2".into());
        cfg
    }

    #[test]
    fn parses_accelerators() {
        assert!(shortcut_id("Ctrl+Alt+M").is_ok());
        assert_eq!(shortcut_id("ctrl+alt+m"), shortcut_id("Ctrl+Alt+M"));
        assert_eq!(shortcut_id("Ctrl+Alt+Nope"), Err(BAD_FORMAT.to_string()));
    }

    #[test]
    fn bindings_list_the_toggle_then_ports() {
        assert_eq!(
            bindings(&config()),
            vec![
                ("Ctrl+Alt+M".to_string(), HotkeyAction::ToggleAll),
                ("Ctrl+Alt+2".to_string(), HotkeyAction::SwitchTo { key: key(), value: 0x1B }),
            ]
        );
    }

    #[test]
    fn detects_conflicts_but_not_with_itself() {
        let cfg = config();
        let usb_c = HotkeyTarget::Port { key: key(), value: 0x1B };
        let dp = HotkeyTarget::Port { key: key(), value: 0x0F };
        assert!(conflicts(&cfg, &dp, "ctrl+alt+2"));
        assert!(conflicts(&cfg, &dp, "Ctrl+Alt+M"));
        assert!(!conflicts(&cfg, &usb_c, "Ctrl+Alt+2"));
        assert!(!conflicts(&cfg, &HotkeyTarget::Toggle, "Ctrl+Alt+M"));
        assert!(!conflicts(&cfg, &dp, "Ctrl+Alt+1"));
    }

    #[test]
    fn get_and_set_reach_the_right_field() {
        let mut cfg = config();
        let dp = HotkeyTarget::Port { key: key(), value: 0x0F };
        set(&mut cfg, &dp, Some("Ctrl+Alt+1".into()));
        assert_eq!(get(&cfg, &dp).as_deref(), Some("Ctrl+Alt+1"));
        set(&mut cfg, &HotkeyTarget::Toggle, None);
        assert_eq!(get(&cfg, &HotkeyTarget::Toggle), None);
    }
}
```

- [ ] **Step 3: 运行测试，确认失败**

Run: `cargo test --manifest-path src-tauri/Cargo.toml hotkeys`
Expected: FAIL（`parses_accelerators`、`bindings_...`、`detects_conflicts_...` 失败）

- [ ] **Step 4: 实现 `shortcut_id`、`bindings`、`conflicts`**

```rust
pub fn shortcut_id(accelerator: &str) -> Result<u32, String> {
    Shortcut::from_str(accelerator.trim())
        .map(|s| s.id())
        .map_err(|_| BAD_FORMAT.to_string())
}

pub fn bindings(cfg: &Config) -> Vec<(String, HotkeyAction)> {
    let mut out = Vec::new();
    if let Some(accelerator) = &cfg.toggle_hotkey {
        out.push((accelerator.clone(), HotkeyAction::ToggleAll));
    }
    for (key, monitor) in &cfg.monitors {
        for (value, port) in &monitor.ports {
            if let (Some(accelerator), Ok(value)) = (&port.hotkey, value.parse::<u16>()) {
                out.push((accelerator.clone(), HotkeyAction::SwitchTo { key: MonitorKey(key.clone()), value }));
            }
        }
    }
    out
}

pub fn conflicts(cfg: &Config, target: &HotkeyTarget, accelerator: &str) -> bool {
    let Ok(id) = shortcut_id(accelerator) else {
        return false;
    };
    bindings(cfg).into_iter().any(|(other, action)| {
        let same_target = match (&action, target) {
            (HotkeyAction::ToggleAll, HotkeyTarget::Toggle) => true,
            (HotkeyAction::SwitchTo { key, value }, HotkeyTarget::Port { key: k, value: v }) => key == k && value == v,
            _ => false,
        };
        !same_target && shortcut_id(&other) == Ok(id)
    })
}
```

- [ ] **Step 5: 运行测试，确认通过**

Run: `cargo test --manifest-path src-tauri/Cargo.toml hotkeys`
Expected: `test result: ok. 4 passed`

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/hotkeys.rs src-tauri/src/lib.rs
git commit -m "feat: hotkey bindings, conflicts and registration"
```

---

### Task 11: 应用状态和面板定位

**Files:**
- Create: `src-tauri/src/app/mod.rs`（本任务只声明子模块）
- Create: `src-tauri/src/app/state.rs`
- Create: `src-tauri/src/app/flyout.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: 在 `lib.rs` 加 `mod app;`，并写 `app/mod.rs`**

```rust
pub mod flyout;
pub mod state;
```

- [ ] **Step 2: 写 `app/state.rs`**

```rust
use std::sync::{Arc, Mutex};

use crate::app::flyout::FlyoutState;
use crate::config::{Config, ConfigStore};
use crate::hotkeys::HotkeyRegistry;
use crate::model::MonitorKey;
use crate::monitor_service::{CapsDecision, MonitorPrefs, MonitorService};
use crate::view::{build_view, View};

pub struct AppState {
    pub config: Arc<Mutex<Config>>,
    pub store: ConfigStore,
    pub service: MonitorService,
    pub hotkeys: HotkeyRegistry,
    pub failed_hotkeys: Mutex<Vec<String>>,
    pub flyout: FlyoutState,
    pub is_laptop: bool,
    pub computer_name: String,
}

impl AppState {
    pub fn view(&self) -> View {
        let cfg = self.config.lock().unwrap();
        let mut view = build_view(&self.service.snapshot(), &cfg, self.is_laptop, &self.computer_name);
        view.failed_hotkeys = self.failed_hotkeys.lock().unwrap().clone();
        view
    }

    /// Change the settings and save them.
    pub fn update<F: FnOnce(&mut Config)>(&self, change: F) -> Result<(), String> {
        let mut cfg = self.config.lock().unwrap();
        change(&mut cfg);
        self.store.save(&cfg).map_err(|e| format!("保存设置失败：{e}"))
    }
}

/// MonitorService's view of the settings file.
pub struct ConfigPrefs {
    pub config: Arc<Mutex<Config>>,
    pub store: ConfigStore,
}

impl MonitorPrefs for ConfigPrefs {
    fn caps_decision(&self, key: &MonitorKey) -> CapsDecision {
        let cfg = self.config.lock().unwrap();
        match cfg.monitor(key) {
            Some(m) if !m.read_capabilities => CapsDecision::Skip,
            Some(m) => match &m.cached_capabilities {
                Some(raw) => CapsDecision::Cached(raw.clone()),
                None => CapsDecision::Read,
            },
            None => CapsDecision::Read,
        }
    }

    fn begin_caps_read(&self, key: &MonitorKey, label: &str) {
        let mut cfg = self.config.lock().unwrap();
        let m = cfg.monitor_mut(key);
        m.label = label.to_string();
        m.caps_read_pending = true;
        let _ = self.store.save(&cfg);
    }

    fn finish_caps_read(&self, key: &MonitorKey, raw: Option<String>) {
        let mut cfg = self.config.lock().unwrap();
        let m = cfg.monitor_mut(key);
        m.caps_read_pending = false;
        // An empty string still counts as "read", so a monitor that never answers isn't asked forever.
        m.cached_capabilities = Some(raw.unwrap_or_default());
        let _ = self.store.save(&cfg);
    }

    fn local_override(&self, key: &MonitorKey) -> Option<u16> {
        self.config.lock().unwrap().monitor(key).and_then(|m| m.local_port_override)
    }
}
```

- [ ] **Step 3: 写 `app/flyout.rs`：先写定位函数的测试**

```rust
//! The tray panel: show, hide, and keep it next to the tray icon.

use std::sync::Mutex;
use std::time::{Duration, Instant};

use tauri::{AppHandle, Emitter, LogicalSize, Manager, PhysicalPosition, WebviewWindow};

use crate::app::state::AppState;

pub const FLYOUT_LABEL: &str = "flyout";
pub const FLYOUT_WIDTH: f64 = 352.0;
pub const DEFAULT_HEIGHT: f64 = 360.0;
const MARGIN: f64 = 12.0;

#[derive(Default)]
pub struct FlyoutState {
    /// Where the tray icon was clicked, in physical pixels.
    anchor: Mutex<Option<(f64, f64)>>,
    last_hidden: Mutex<Option<Instant>>,
    /// Content height reported by the page, in logical pixels.
    height: Mutex<Option<f64>>,
}

/// Top-left corner for a panel of `size` (physical px) shown for a click at `click`,
/// inside the monitor work area `work` = (x, y, width, height).
/// Taskbar at the bottom → panel above the click; menu bar at the top → panel below it.
pub fn origin(_click: (f64, f64), _work: (i32, i32, u32, u32), _size: (u32, u32), _margin: i32) -> (i32, i32) {
    (0, 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bottom_taskbar_puts_the_panel_above_the_click() {
        assert_eq!(origin((1880.0, 1060.0), (0, 0, 1920, 1032), (352, 400), 12), (1920 - 352 - 12, 1032 - 400 - 12));
    }

    #[test]
    fn top_menu_bar_puts_the_panel_below_the_click() {
        assert_eq!(origin((800.0, 10.0), (0, 25, 1440, 875), (352, 400), 12), (800 - 176, 25 + 12));
    }

    #[test]
    fn stays_inside_the_left_edge() {
        assert_eq!(origin((50.0, 1060.0), (0, 0, 1920, 1032), (352, 400), 12).0, 12);
    }

    #[test]
    fn works_on_a_secondary_monitor() {
        assert_eq!(origin((3800.0, 1060.0), (1920, 0, 1920, 1032), (352, 400), 12), (1920 + 1920 - 352 - 12, 620));
    }
}
```

- [ ] **Step 4: 运行测试，确认失败**

Run: `cargo test --manifest-path src-tauri/Cargo.toml flyout`
Expected: FAIL

- [ ] **Step 5: 实现 `origin` 和面板的显示/隐藏**

把 `origin` 替换为：

```rust
pub fn origin(click: (f64, f64), work: (i32, i32, u32, u32), size: (u32, u32), margin: i32) -> (i32, i32) {
    let (wx, wy, ww, wh) = (work.0, work.1, work.2 as i32, work.3 as i32);
    let (w, h) = (size.0 as i32, size.1 as i32);
    let min_x = wx + margin;
    let max_x = (wx + ww - w - margin).max(min_x);
    let x = (click.0.round() as i32 - w / 2).clamp(min_x, max_x);
    let min_y = wy + margin;
    let max_y = (wy + wh - h - margin).max(min_y);
    let taskbar_at_bottom = click.1.round() as i32 >= wy + wh / 2;
    let y = if taskbar_at_bottom { max_y } else { min_y };
    (x, y)
}
```

在 `#[cfg(test)]` 之前加入：

```rust
fn window(app: &AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window(FLYOUT_LABEL)
}

/// Left click on the tray icon.
pub fn toggle_from_tray(app: &AppHandle, click: (f64, f64)) {
    let Some(win) = window(app) else { return };
    if win.is_visible().unwrap_or(false) {
        hide(app);
        return;
    }
    // Clicking the tray icon while the panel is open first blurs it (which hides it),
    // then delivers the click. Don't reopen it in that case.
    let state = app.state::<AppState>();
    let just_hidden = state
        .flyout
        .last_hidden
        .lock()
        .unwrap()
        .map(|t| t.elapsed() < Duration::from_millis(300))
        .unwrap_or(false);
    if just_hidden {
        return;
    }
    *state.flyout.anchor.lock().unwrap() = Some(click);
    show(app);
}

/// Show the panel near the bottom-right corner of the primary screen (used by single-instance).
pub fn show_default(app: &AppHandle) {
    if let Ok(Some(monitor)) = app.primary_monitor() {
        let wa = monitor.work_area();
        let corner = (
            (wa.position.x + wa.size.width as i32 - 1) as f64,
            (wa.position.y + wa.size.height as i32 - 1) as f64,
        );
        *app.state::<AppState>().flyout.anchor.lock().unwrap() = Some(corner);
    }
    show(app);
}

pub fn show(app: &AppHandle) {
    let Some(win) = window(app) else { return };
    place(app, &win);
    let _ = win.show();
    let _ = win.set_focus();
    let state = app.state::<AppState>();
    state.service.refresh();
    let _ = app.emit_to(FLYOUT_LABEL, "view-changed", state.view());
}

pub fn hide(app: &AppHandle) {
    if let Some(win) = window(app) {
        let _ = win.hide();
    }
    *app.state::<AppState>().flyout.last_hidden.lock().unwrap() = Some(Instant::now());
}

/// The page reports its content height after every render.
pub fn fit(app: &AppHandle, height: f64) {
    *app.state::<AppState>().flyout.height.lock().unwrap() = Some(height.clamp(120.0, 900.0));
    if let Some(win) = window(app) {
        place(app, &win);
    }
}

fn place(app: &AppHandle, win: &WebviewWindow) {
    let state = app.state::<AppState>();
    let anchor = state.flyout.anchor.lock().unwrap().unwrap_or((0.0, 0.0));
    let height = state.flyout.height.lock().unwrap().unwrap_or(DEFAULT_HEIGHT);
    let monitor = app
        .monitor_from_point(anchor.0, anchor.1)
        .ok()
        .flatten()
        .or_else(|| app.primary_monitor().ok().flatten());
    let Some(monitor) = monitor else { return };
    let scale = monitor.scale_factor();
    let _ = win.set_size(LogicalSize::new(FLYOUT_WIDTH, height));
    let wa = monitor.work_area();
    let size = ((FLYOUT_WIDTH * scale).round() as u32, (height * scale).round() as u32);
    let (x, y) = origin(
        anchor,
        (wa.position.x, wa.position.y, wa.size.width, wa.size.height),
        size,
        (MARGIN * scale).round() as i32,
    );
    let _ = win.set_position(PhysicalPosition::new(x, y));
}
```

- [ ] **Step 6: 运行测试，确认通过**

Run: `cargo test --manifest-path src-tauri/Cargo.toml flyout`
Expected: `test result: ok. 4 passed`

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/app src-tauri/src/lib.rs
git commit -m "feat: app state, config-backed prefs and flyout placement"
```

---

### Task 12: 屏幕提示、托盘、动作、命令，并把应用连起来

**Files:**
- Create: `src-tauri/src/app/osd.rs`
- Create: `src-tauri/src/app/tray.rs`
- Create: `src-tauri/src/app/actions.rs`
- Create: `src-tauri/src/app/windows.rs`
- Create: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/app/mod.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: 写 `app/osd.rs`**

```rust
//! The on-screen prompt shown when switching.

use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition};

use crate::view::{MonitorView, PortView};

pub const OSD_LABEL: &str = "osd";
pub const OSD_WIDTH: f64 = 260.0;
pub const OSD_HEIGHT: f64 = 210.0;
const VISIBLE_FOR: Duration = Duration::from_millis(1500);

/// Each show gets a number; only the latest one may hide the window.
static GENERATION: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OsdPayload {
    pub title: String,
    pub name: String,
    pub detail: String,
    pub icon: String,
}

pub fn for_port(title: &str, monitor: &MonitorView, port: &PortView) -> OsdPayload {
    OsdPayload {
        title: title.to_string(),
        name: port.display_name.clone(),
        detail: format!("{} · {}", port.port_label, monitor.label),
        icon: port.icon.to_string(),
    }
}

pub fn show(app: &AppHandle, payload: OsdPayload) {
    let Some(win) = app.get_webview_window(OSD_LABEL) else { return };
    // On the screen the mouse is on, centred, a little below the top.
    if let Ok(cursor) = app.cursor_position() {
        if let Ok(Some(monitor)) = app.monitor_from_point(cursor.x, cursor.y) {
            let wa = monitor.work_area();
            let width = (OSD_WIDTH * monitor.scale_factor()).round() as i32;
            let x = wa.position.x + (wa.size.width as i32 - width) / 2;
            let y = wa.position.y + (wa.size.height as f64 * 0.16).round() as i32;
            let _ = win.set_position(PhysicalPosition::new(x, y));
        }
    }
    let _ = app.emit_to(OSD_LABEL, "osd-show", payload);
    let _ = win.show();
    let generation = GENERATION.fetch_add(1, Ordering::SeqCst) + 1;
    let app = app.clone();
    thread::spawn(move || {
        thread::sleep(VISIBLE_FOR);
        if GENERATION.load(Ordering::SeqCst) == generation {
            if let Some(win) = app.get_webview_window(OSD_LABEL) {
                let _ = win.hide();
            }
        }
    });
}
```

- [ ] **Step 2: 写 `app/windows.rs`**

```rust
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder, WindowEvent};

use crate::app::{flyout, osd};

pub const SETTINGS_LABEL: &str = "settings";

pub fn create_flyout(app: &AppHandle) -> tauri::Result<()> {
    let win = WebviewWindowBuilder::new(app, flyout::FLYOUT_LABEL, WebviewUrl::App("flyout.html".into()))
        .title("MonitorSwitch")
        .inner_size(flyout::FLYOUT_WIDTH, flyout::DEFAULT_HEIGHT)
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .resizable(false)
        .skip_taskbar(true)
        .always_on_top(true)
        .visible(false)
        .build()?;
    let handle = app.clone();
    win.on_window_event(move |event| {
        if let WindowEvent::Focused(false) = event {
            flyout::hide(&handle);
        }
    });
    Ok(())
}

pub fn create_osd(app: &AppHandle) -> tauri::Result<()> {
    let win = WebviewWindowBuilder::new(app, osd::OSD_LABEL, WebviewUrl::App("osd.html".into()))
        .title("MonitorSwitch")
        .inner_size(osd::OSD_WIDTH, osd::OSD_HEIGHT)
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .resizable(false)
        .skip_taskbar(true)
        .always_on_top(true)
        .focusable(false)
        .focused(false)
        .visible(false)
        .build()?;
    win.set_ignore_cursor_events(true)?;
    Ok(())
}

/// Must not be called from a synchronous command on Windows (window creation would deadlock);
/// the `open_settings` command is async for that reason.
pub fn open_settings(app: &AppHandle) {
    flyout::hide(app);
    if let Some(win) = app.get_webview_window(SETTINGS_LABEL) {
        let _ = win.unminimize();
        let _ = win.show();
        let _ = win.set_focus();
        return;
    }
    let _ = WebviewWindowBuilder::new(app, SETTINGS_LABEL, WebviewUrl::App("settings.html".into()))
        .title("MonitorSwitch 设置")
        .inner_size(520.0, 680.0)
        .build();
}
```

- [ ] **Step 3: 写 `app/tray.rs`（含悬停提示的测试）**

```rust
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, Wry};

use crate::app::{actions, flyout, state::AppState, windows};
use crate::model::MonitorKey;
use crate::view::{MonitorView, PortView, View};

pub const TRAY_ID: &str = "main";

/// One line per monitor: "Dell U2723QE · 公司笔记本（USB-C）".
pub fn tooltip_text(view: &View) -> String {
    if view.monitors.is_empty() {
        return "MonitorSwitch · 没找到能控制的显示器".into();
    }
    view.monitors
        .iter()
        .map(|m| {
            let current = m
                .current
                .and_then(|c| m.ports.iter().find(|p| p.value == c))
                .map(|p| format!("{}（{}）", p.display_name, p.port_label))
                .unwrap_or_else(|| "读不到当前输入".into());
            format!("{} · {}", m.label, current)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn port_item(app: &AppHandle, monitor: &MonitorView, port: &PortView) -> tauri::Result<CheckMenuItem<Wry>> {
    CheckMenuItem::with_id(
        app,
        format!("switch|{}|{}", monitor.key.0, port.value),
        format!("{}（{}）", port.display_name, port.port_label),
        true,
        port.is_current,
        None::<&str>,
    )
}

/// One monitor → its inputs at the top level; several → one submenu each.
pub fn build_menu(app: &AppHandle, view: &View) -> tauri::Result<Menu<Wry>> {
    let menu = Menu::new(app)?;
    if let [monitor] = view.monitors.as_slice() {
        for port in &monitor.ports {
            menu.append(&port_item(app, monitor, port)?)?;
        }
    } else {
        for monitor in &view.monitors {
            let submenu = Submenu::new(app, &monitor.label, true)?;
            for port in &monitor.ports {
                submenu.append(&port_item(app, monitor, port)?)?;
            }
            menu.append(&submenu)?;
        }
    }
    if !view.monitors.is_empty() {
        menu.append(&PredefinedMenuItem::separator(app)?)?;
    }
    menu.append(&MenuItem::with_id(app, "settings", "设置", true, None::<&str>)?)?;
    menu.append(&MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?)?;
    Ok(menu)
}

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let view = app.state::<AppState>().view();
    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip(tooltip_text(&view))
        .menu(&build_menu(app, &view)?)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| on_menu(app, event.id().as_ref()))
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                position,
                ..
            } = event
            {
                flyout::toggle_from_tray(tray.app_handle(), (position.x, position.y));
            }
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    Ok(())
}

pub fn update(app: &AppHandle, view: &View) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else { return };
    let _ = tray.set_tooltip(Some(tooltip_text(view)));
    if let Ok(menu) = build_menu(app, view) {
        let _ = tray.set_menu(Some(menu));
    }
}

fn on_menu(app: &AppHandle, id: &str) {
    match id {
        "settings" => windows::open_settings(app),
        "quit" => app.exit(0),
        other => {
            let Some(rest) = other.strip_prefix("switch|") else { return };
            let Some((key, value)) = rest.rsplit_once('|') else { return };
            if let Ok(value) = value.parse::<u16>() {
                actions::perform_switch(app, MonitorKey(key.to_string()), value);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::model::ConnectionKind;
    use crate::monitor_service::MonitorState;
    use crate::view::build_view;

    #[test]
    fn tooltip_lists_each_monitor() {
        let mut cfg = Config::default();
        let key = MonitorKey("DEL426A#UID1".into());
        cfg.monitor_mut(&key).port_mut(0x1B).name = Some("公司笔记本".into());
        let state = MonitorState {
            key,
            label: "Dell U2723QE".into(),
            connection: ConnectionKind::DisplayPort,
            inputs: vec![0x0F, 0x11, 0x1B],
            current: Some(0x1B),
            local_input: Some(0x0F),
            error: None,
        };
        let view = build_view(&[state], &cfg, false, "PC");
        assert_eq!(tooltip_text(&view), "Dell U2723QE · 公司笔记本（USB-C）");
    }

    #[test]
    fn tooltip_without_monitors() {
        let view = build_view(&[], &Config::default(), false, "PC");
        assert_eq!(tooltip_text(&view), "MonitorSwitch · 没找到能控制的显示器");
    }
}
```

- [ ] **Step 4: 写 `app/actions.rs`**

```rust
//! What happens when the user switches, presses a hotkey, or the monitor changes.

use std::thread;
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_autostart::ManagerExt;

use crate::app::{osd, state::AppState, tray};
use crate::hotkeys::{self, HotkeyAction, HotkeyTarget, IN_USE_ELSEWHERE, IN_USE_HERE};
use crate::model::MonitorKey;
use crate::monitor_service::ServiceEvent;
use crate::toggle::toggle_target;

/// How long the "正在切换到" prompt stays on this screen before the switch is sent (spec 5.2).
const PRE_SWITCH_PROMPT: Duration = Duration::from_millis(600);

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SwitchFailedPayload {
    pub key: String,
    pub value: u16,
    pub reason: String,
}

pub fn perform_switch(app: &AppHandle, key: MonitorKey, value: u16) {
    let state = app.state::<AppState>();
    let view = state.view();
    let Some(monitor) = view.monitors.iter().find(|m| m.key == key) else { return };
    let port = monitor.ports.iter().find(|p| p.value == value);
    let leaving_this_computer = !port.map(|p| p.is_local).unwrap_or(false);
    if view.show_osd && leaving_this_computer {
        if let Some(port) = port {
            osd::show(app, osd::for_port("正在切换到", monitor, port));
        }
        let app = app.clone();
        thread::spawn(move || {
            thread::sleep(PRE_SWITCH_PROMPT);
            app.state::<AppState>().service.switch(key, value);
        });
    } else {
        state.service.switch(key, value);
    }
}

/// The toggle hotkey: every monitor with a toggle pair switches to the other input of its pair.
pub fn toggle_all(app: &AppHandle) {
    let view = app.state::<AppState>().view();
    for monitor in &view.monitors {
        if let Some(pair) = monitor.toggle_pair {
            perform_switch(app, monitor.key.clone(), toggle_target(monitor.current, pair));
        }
    }
}

pub fn on_hotkey(app: &AppHandle, shortcut_id: u32) {
    let Some(state) = app.try_state::<AppState>() else { return };
    match state.hotkeys.action_for(shortcut_id) {
        Some(HotkeyAction::ToggleAll) => toggle_all(app),
        Some(HotkeyAction::SwitchTo { key, value }) => perform_switch(app, key, value),
        None => {}
    }
}

/// Runs on the MonitorService worker thread.
pub fn handle_service_event(app: &AppHandle, event: ServiceEvent) {
    let Some(state) = app.try_state::<AppState>() else { return };
    match event {
        ServiceEvent::Snapshot(_) => publish_view(app),
        ServiceEvent::InputChanged { key, to, local_input, .. } => {
            let view = state.view();
            // The monitor now shows this computer: confirm it on this computer's screen.
            if view.show_osd && local_input == Some(to) {
                if let Some(monitor) = view.monitors.iter().find(|m| m.key == key) {
                    if let Some(port) = monitor.ports.iter().find(|p| p.value == to) {
                        osd::show(app, osd::for_port("已切换到", monitor, port));
                    }
                }
            }
            publish_view(app);
        }
        ServiceEvent::SwitchFailed { key, value, reason } => {
            let _ = app.emit("switch-failed", SwitchFailedPayload { key: key.0, value, reason });
        }
    }
}

/// Push the current view to every window and refresh the tray.
pub fn publish_view(app: &AppHandle) {
    let Some(state) = app.try_state::<AppState>() else { return };
    let view = state.view();
    tray::update(app, &view);
    let _ = app.emit("view-changed", view);
}

/// Register every configured hotkey again and remember which ones the OS refused.
pub fn apply_hotkeys(app: &AppHandle) -> Vec<String> {
    let state = app.state::<AppState>();
    let bindings = hotkeys::bindings(&state.config.lock().unwrap());
    let failed = state.hotkeys.apply(app, bindings);
    *state.failed_hotkeys.lock().unwrap() = failed.clone();
    failed
}

pub fn set_hotkey(app: &AppHandle, target: HotkeyTarget, accelerator: Option<String>) -> Result<(), String> {
    let state = app.state::<AppState>();
    let accelerator = accelerator.map(|a| a.trim().to_string()).filter(|a| !a.is_empty());
    if let Some(a) = &accelerator {
        hotkeys::shortcut_id(a)?;
    }
    let previous = {
        let mut cfg = state.config.lock().unwrap();
        if let Some(a) = &accelerator {
            if hotkeys::conflicts(&cfg, &target, a) {
                return Err(IN_USE_HERE.into());
            }
        }
        let previous = hotkeys::get(&cfg, &target);
        hotkeys::set(&mut cfg, &target, accelerator.clone());
        previous
    };
    let failed = apply_hotkeys(app);
    if let Some(a) = &accelerator {
        if failed.contains(a) {
            hotkeys::set(&mut state.config.lock().unwrap(), &target, previous);
            apply_hotkeys(app);
            return Err(IN_USE_ELSEWHERE.into());
        }
    }
    state.update(|_| {})?;
    publish_view(app);
    Ok(())
}

/// Make the OS autostart entry match the setting. Skipped in debug builds, which would
/// register the development executable.
pub fn sync_autostart(app: &AppHandle) {
    if cfg!(debug_assertions) {
        return;
    }
    let want = app.state::<AppState>().config.lock().unwrap().autostart;
    let launcher = app.autolaunch();
    let _ = if want { launcher.enable() } else { launcher.disable() };
}
```

- [ ] **Step 5: 写 `commands.rs`**

```rust
//! Commands the pages call with `invoke`. Argument names arrive camelCase from JavaScript.

use tauri::{AppHandle, Manager, State};

use crate::app::{actions, flyout, state::AppState, windows};
use crate::hotkeys::HotkeyTarget;
use crate::model::MonitorKey;
use crate::view::View;

#[tauri::command]
pub fn get_view(state: State<'_, AppState>) -> View {
    state.view()
}

#[tauri::command]
pub fn refresh(state: State<'_, AppState>) {
    state.service.refresh();
}

#[tauri::command]
pub fn switch_input(app: AppHandle, key: String, value: u16) {
    actions::perform_switch(&app, MonitorKey(key), value);
}

#[tauri::command]
pub fn toggle(app: AppHandle) {
    actions::toggle_all(&app);
}

#[tauri::command]
pub fn set_port_name(app: AppHandle, key: String, value: u16, name: Option<String>) -> Result<(), String> {
    let name = name.map(|n| n.trim().to_string()).filter(|n| !n.is_empty());
    app.state::<AppState>()
        .update(|cfg| cfg.monitor_mut(&MonitorKey(key)).port_mut(value).name = name)?;
    actions::publish_view(&app);
    Ok(())
}

#[tauri::command]
pub fn set_port_hotkey(app: AppHandle, key: String, value: u16, accelerator: Option<String>) -> Result<(), String> {
    actions::set_hotkey(&app, HotkeyTarget::Port { key: MonitorKey(key), value }, accelerator)
}

#[tauri::command]
pub fn set_toggle_hotkey(app: AppHandle, accelerator: Option<String>) -> Result<(), String> {
    actions::set_hotkey(&app, HotkeyTarget::Toggle, accelerator)
}

#[tauri::command]
pub fn set_toggle_pair(app: AppHandle, key: String, pair: Option<[u16; 2]>) -> Result<(), String> {
    if let Some([a, b]) = pair {
        if a == b {
            return Err("两个接口要不一样".into());
        }
    }
    app.state::<AppState>()
        .update(|cfg| cfg.monitor_mut(&MonitorKey(key)).toggle_pair = pair)?;
    actions::publish_view(&app);
    Ok(())
}

#[tauri::command]
pub fn set_local_override(app: AppHandle, key: String, value: Option<u16>) -> Result<(), String> {
    let state = app.state::<AppState>();
    state.update(|cfg| cfg.monitor_mut(&MonitorKey(key)).local_port_override = value)?;
    state.service.refresh();
    Ok(())
}

#[tauri::command]
pub fn set_read_capabilities(app: AppHandle, key: String, enabled: bool) -> Result<(), String> {
    let state = app.state::<AppState>();
    state.update(|cfg| {
        let m = cfg.monitor_mut(&MonitorKey(key));
        m.read_capabilities = enabled;
        m.caps_read_pending = false;
        if enabled {
            m.cached_capabilities = None;
        }
    })?;
    state.service.refresh();
    Ok(())
}

#[tauri::command]
pub fn set_option(app: AppHandle, name: String, value: bool) -> Result<(), String> {
    let state = app.state::<AppState>();
    match name.as_str() {
        "showOsd" => state.update(|cfg| cfg.show_osd = value)?,
        "lanDiscovery" => state.update(|cfg| cfg.lan_discovery = value)?,
        "autostart" => {
            state.update(|cfg| cfg.autostart = value)?;
            actions::sync_autostart(&app);
        }
        other => return Err(format!("unknown option {other}")),
    }
    actions::publish_view(&app);
    Ok(())
}

/// Async: creating a window from a synchronous command deadlocks on Windows.
#[tauri::command]
pub async fn open_settings(app: AppHandle) {
    windows::open_settings(&app);
}

#[tauri::command]
pub fn hide_flyout(app: AppHandle) {
    flyout::hide(&app);
}

#[tauri::command]
pub fn fit_flyout(app: AppHandle, height: f64) {
    flyout::fit(&app, height);
}
```

- [ ] **Step 6: 写完整的 `app/mod.rs`**

```rust
pub mod actions;
pub mod flyout;
pub mod osd;
pub mod state;
pub mod tray;
pub mod windows;

use std::sync::{Arc, Mutex};

use tauri::{AppHandle, Manager};

use crate::config::ConfigStore;
use crate::hotkeys::HotkeyRegistry;
use crate::monitor_service::{MonitorService, Timing};
use crate::platform::{self, NativeBackend};
use flyout::FlyoutState;
use state::{AppState, ConfigPrefs};

pub fn setup(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let store = ConfigStore::new(app.path().config_dir()?.join("MonitorSwitch").join("config.json"));
    let mut cfg = store.load();
    // A capabilities read that never finished crashed us last time (spec 6.4).
    if !cfg.recover_interrupted_caps_reads().is_empty() {
        let _ = store.save(&cfg);
    }
    let config = Arc::new(Mutex::new(cfg));
    let prefs = Arc::new(ConfigPrefs { config: config.clone(), store: store.clone() });
    let is_laptop = platform::is_laptop();
    let handle = app.clone();
    let service = MonitorService::start(
        Box::new(NativeBackend::new()),
        prefs,
        is_laptop,
        Timing::production(),
        move |event| actions::handle_service_event(&handle, event),
    );
    app.manage(AppState {
        config,
        store,
        service,
        hotkeys: HotkeyRegistry::default(),
        failed_hotkeys: Mutex::new(Vec::new()),
        flyout: FlyoutState::default(),
        is_laptop,
        computer_name: platform::computer_name(),
    });

    windows::create_flyout(app)?;
    windows::create_osd(app)?;
    tray::create(app)?;
    actions::apply_hotkeys(app);
    actions::sync_autostart(app);
    actions::publish_view(app);
    Ok(())
}
```

- [ ] **Step 7: 写完整的 `lib.rs`**

```rust
mod app;
mod commands;
pub mod config;
pub mod ddc;
pub mod hotkeys;
pub mod local_port;
pub mod model;
pub mod monitor_service;
pub mod platform;
pub mod toggle;
pub mod view;

use tauri_plugin_autostart::MacosLauncher;
use tauri_plugin_global_shortcut::ShortcutState;

pub fn run() {
    tauri::Builder::default()
        // Must be the first plugin: a second launch just opens the running app's panel.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            app::flyout::show_default(app);
        }))
        .plugin(
            tauri_plugin_global_shortcut::Builder::<tauri::Wry>::new()
                .with_handler(|app, shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        app::actions::on_hotkey(app, shortcut.id());
                    }
                })
                .build(),
        )
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, None))
        .invoke_handler(tauri::generate_handler![
            commands::get_view,
            commands::refresh,
            commands::switch_input,
            commands::toggle,
            commands::set_port_name,
            commands::set_port_hotkey,
            commands::set_toggle_hotkey,
            commands::set_toggle_pair,
            commands::set_local_override,
            commands::set_read_capabilities,
            commands::set_option,
            commands::open_settings,
            commands::hide_flyout,
            commands::fit_flyout,
        ])
        .setup(|app| {
            app::setup(app.handle())?;
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building MonitorSwitch")
        .run(|_app, event| {
            // Closing the settings window must not quit the tray app; "退出" calls exit(0).
            if let tauri::RunEvent::ExitRequested { api, code, .. } = event {
                if code.is_none() {
                    api.prevent_exit();
                }
            }
        });
}
```

- [ ] **Step 8: 编译并跑全部测试**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`
Expected: 全部通过（59 个测试，1 个 ignored）。如果编译报某个 Tauri API 的签名不符，以 `~/.cargo/registry/src/index.crates.io-*/tauri-2.12.*/src/` 里的源码为准修正调用，不改变行为。

- [ ] **Step 9: Commit**

```bash
git add src-tauri/src
git commit -m "feat: wire tray, flyout, OSD, hotkeys and commands into the app"
```

---

### Task 13: 界面公共部分和托盘面板

**Files:**
- Create: `src/shared/api.ts`、`src/shared/hotkey.ts`、`src/shared/icons.ts`
- Modify: `src/shared/theme.css`、`src/flyout/main.ts`、`src/flyout/flyout.css`

- [ ] **Step 1: 写 `src/shared/api.ts`**

```ts
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type Icon = "desktop" | "laptop" | "plug";

export interface PortView {
  value: number;
  portLabel: string;
  name: string | null;
  displayName: string;
  isLocal: boolean;
  isCurrent: boolean;
  hotkey: string | null;
  icon: Icon;
}

export interface MonitorView {
  key: string;
  label: string;
  connection: "hdmi" | "displayPort" | "usbTunnel" | "other";
  current: number | null;
  currentName: string | null;
  error: string | null;
  ports: PortView[];
  togglePair: [number, number] | null;
  togglePairIsDefault: boolean;
  localOverride: number | null;
  readCapabilities: boolean;
}

export interface View {
  monitors: MonitorView[];
  showOsd: boolean;
  autostart: boolean;
  lanDiscovery: boolean;
  toggleHotkey: string | null;
  computerName: string;
  failedHotkeys: string[];
}

export interface SwitchFailed {
  key: string;
  value: number;
  reason: string;
}

export type OptionName = "showOsd" | "autostart" | "lanDiscovery";

export const api = {
  getView: () => invoke<View>("get_view"),
  refresh: () => invoke<void>("refresh"),
  switchInput: (key: string, value: number) => invoke<void>("switch_input", { key, value }),
  toggle: () => invoke<void>("toggle"),
  setPortName: (key: string, value: number, name: string | null) =>
    invoke<void>("set_port_name", { key, value, name }),
  setPortHotkey: (key: string, value: number, accelerator: string | null) =>
    invoke<void>("set_port_hotkey", { key, value, accelerator }),
  setToggleHotkey: (accelerator: string | null) => invoke<void>("set_toggle_hotkey", { accelerator }),
  setTogglePair: (key: string, pair: [number, number] | null) => invoke<void>("set_toggle_pair", { key, pair }),
  setLocalOverride: (key: string, value: number | null) => invoke<void>("set_local_override", { key, value }),
  setReadCapabilities: (key: string, enabled: boolean) => invoke<void>("set_read_capabilities", { key, enabled }),
  setOption: (name: OptionName, value: boolean) => invoke<void>("set_option", { name, value }),
  openSettings: () => invoke<void>("open_settings"),
  hideFlyout: () => invoke<void>("hide_flyout"),
  fitFlyout: (height: number) => invoke<void>("fit_flyout", { height }),
};

export function onView(callback: (view: View) => void): Promise<UnlistenFn> {
  return listen<View>("view-changed", (e) => callback(e.payload));
}

export function onSwitchFailed(callback: (failure: SwitchFailed) => void): Promise<UnlistenFn> {
  return listen<SwitchFailed>("switch-failed", (e) => callback(e.payload));
}

const ENTITIES: Record<string, string> = { "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" };

/** Escape text before putting it into innerHTML. Names come from the user. */
export function esc(text: string): string {
  return text.replace(/[&<>"']/g, (c) => ENTITIES[c]);
}
```

- [ ] **Step 2: 写 `src/shared/hotkey.ts`**

```ts
const MODIFIER_KEYS = new Set(["Control", "Alt", "Shift", "Meta"]);

/**
 * Turn a keydown into an accelerator such as "Ctrl+Alt+M".
 * Returns null while only modifiers are held, or when no modifier is held
 * (a bare letter as a global hotkey would swallow normal typing).
 */
export function acceleratorFromEvent(e: KeyboardEvent): string | null {
  if (MODIFIER_KEYS.has(e.key)) return null;
  const parts: string[] = [];
  if (e.ctrlKey) parts.push("Ctrl");
  if (e.altKey) parts.push("Alt");
  if (e.shiftKey) parts.push("Shift");
  if (e.metaKey) parts.push("Super");
  if (parts.length === 0) return null;
  let key = e.code;
  if (key.startsWith("Key")) key = key.slice(3);
  else if (key.startsWith("Digit")) key = key.slice(5);
  parts.push(key);
  return parts.join("+");
}

/** "Ctrl+Alt+M" → "Ctrl + Alt + M". */
export function prettyAccelerator(accelerator: string | null): string {
  return accelerator ? accelerator.split("+").join(" + ") : "";
}
```

- [ ] **Step 3: 写 `src/shared/icons.ts`**

```ts
const BODIES: Record<string, string> = {
  desktop: '<rect x="7" y="3" width="10" height="18" rx="2"/><path d="M10 7h4M10 10h4"/><circle cx="12" cy="17" r="1"/>',
  laptop: '<rect x="5" y="5" width="14" height="10" rx="1.5"/><path d="M3 19h18"/>',
  plug: '<path d="M5 9h14v4l-2 3H7l-2-3z"/><path d="M9 9V6M15 9V6"/>',
  toggle: '<path d="M4 8h13l-3-3M20 16H7l3 3"/>',
  settings:
    '<path d="M4 6h9M17 6h3M4 12h3M11 12h9M4 18h11M19 18h1"/><circle cx="15" cy="6" r="2"/><circle cx="9" cy="12" r="2"/><circle cx="17" cy="18" r="2"/>',
};

/** Outline icon as an inline SVG string, coloured by the surrounding text colour. */
export function icon(name: string, size = 22): string {
  const body = BODIES[name] ?? BODIES.plug;
  return `<svg width="${size}" height="${size}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${body}</svg>`;
}
```

- [ ] **Step 4: 写 `src/shared/theme.css`**

```css
:root {
  --bg: #202124;
  --tile: #2c2d31;
  --tile-2: #34363b;
  --text: #f1f3f5;
  --muted: #a6abb2;
  --faint: #8e949c;
  --accent: #2563eb;
  --border: rgba(255, 255, 255, 0.08);
  --danger: #f87171;
  font-family: Inter, "Segoe UI", "Microsoft YaHei UI", "PingFang SC", system-ui, sans-serif;
  font-size: 14px;
  color: var(--text);
}

* {
  box-sizing: border-box;
}

html,
body {
  margin: 0;
  background: transparent;
  user-select: none;
  -webkit-user-select: none;
}

button {
  font: inherit;
  color: inherit;
  cursor: pointer;
}

input,
select {
  user-select: text;
  -webkit-user-select: text;
}
```

- [ ] **Step 5: 写 `src/flyout/flyout.css`**

```css
.panel {
  width: 352px;
  padding: 16px;
  background: var(--bg);
  border: 1px solid var(--border);
  border-radius: 16px;
  display: flex;
  flex-direction: column;
  gap: 14px;
}

.tabs {
  display: grid;
  grid-auto-flow: column;
  grid-auto-columns: minmax(0, 1fr);
  gap: 4px;
  padding: 4px;
  border-radius: 12px;
  background: var(--tile);
}

.tab {
  height: 32px;
  border: 0;
  border-radius: 9px;
  background: transparent;
  color: var(--muted);
  font-size: 12px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.tab[aria-selected="true"] {
  background: #45474d;
  color: #fff;
  font-weight: 600;
}

.head {
  display: flex;
  align-items: center;
  gap: 10px;
}

.titles {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.title {
  font-size: 15px;
  font-weight: 600;
}

.sub {
  font-size: 12px;
  color: var(--muted);
}

.icon-btn {
  width: 34px;
  height: 34px;
  border: 0;
  border-radius: 10px;
  background: var(--tile);
  color: #d5d9de;
  display: flex;
  align-items: center;
  justify-content: center;
}

.error {
  font-size: 12px;
  line-height: 1.5;
  color: #fecaca;
  background: #3a2224;
  border-radius: 10px;
  padding: 10px 12px;
}

.grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 10px;
}

.tile {
  min-height: 112px;
  display: flex;
  flex-direction: column;
  justify-content: space-between;
  align-items: flex-start;
  gap: 10px;
  padding: 12px;
  border: 0;
  border-radius: 14px;
  background: var(--tile);
  text-align: left;
}

.tile:hover {
  background: #33353a;
}

.tile.current,
.tile.current:hover {
  background: var(--accent);
}

.tile.unnamed {
  background: transparent;
  border: 1px dashed rgba(255, 255, 255, 0.18);
}

.top {
  width: 100%;
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.glyph {
  width: 36px;
  height: 36px;
  border-radius: 10px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--tile-2);
  color: #e2e5e9;
}

.tile.current .glyph {
  background: rgba(255, 255, 255, 0.2);
  color: #fff;
}

.tile.unnamed .glyph {
  color: var(--faint);
}

.badge {
  font-size: 11px;
  padding: 2px 8px;
  border-radius: 10px;
  background: var(--tile-2);
  color: #c9cdd2;
}

.tile.current .badge {
  background: rgba(255, 255, 255, 0.22);
  color: #fff;
}

.labels {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
  max-width: 100%;
}

.name {
  font-size: 15px;
  font-weight: 600;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.tile.unnamed .name {
  color: var(--muted);
}

.port {
  font-size: 12px;
  color: var(--muted);
}

.tile.current .port {
  color: rgba(255, 255, 255, 0.85);
}

.empty {
  margin: 0;
  font-size: 13px;
  line-height: 1.6;
  color: var(--muted);
}
```

- [ ] **Step 6: 写 `src/flyout/main.ts`**

```ts
import { api, esc, onSwitchFailed, onView, type MonitorView, type PortView, type View } from "../shared/api";
import { prettyAccelerator } from "../shared/hotkey";
import { icon } from "../shared/icons";

const panel = document.getElementById("panel") as HTMLElement;
let view: View | null = null;
let selectedKey: string | null = null;
let failure: string | null = null;
let failureTimer: number | undefined;

function selectedMonitor(v: View): MonitorView | undefined {
  return v.monitors.find((m) => m.key === selectedKey) ?? v.monitors[0];
}

const settingsButton = () =>
  `<button class="icon-btn" data-settings aria-label="打开设置">${icon("settings", 18)}</button>`;

function tile(monitor: MonitorView, port: PortView): string {
  const unnamed = !port.name && !port.isLocal;
  const classes = ["tile", port.isCurrent ? "current" : "", unnamed ? "unnamed" : ""].join(" ").trim();
  const badge = port.isLocal ? `<span class="badge">本机</span>` : "";
  return `<button class="${classes}" data-key="${esc(monitor.key)}" data-value="${port.value}" aria-pressed="${port.isCurrent}">
    <span class="top"><span class="glyph">${icon(port.icon)}</span>${badge}</span>
    <span class="labels"><span class="name">${esc(port.displayName)}</span><span class="port">${esc(port.portLabel)}</span></span>
  </button>`;
}

function render(): void {
  if (!view) return;
  const monitor = selectedMonitor(view);
  if (!monitor) {
    panel.innerHTML = `<div class="head"><div class="titles"><span class="title">MonitorSwitch</span></div>${settingsButton()}</div>
      <p class="empty">没找到能控制的显示器。确认显示器菜单里 DDC/CI 已开启，并且接的是外接显示器。</p>`;
    afterRender();
    return;
  }
  selectedKey = monitor.key;
  const many = view.monitors.length > 1;
  const tabs = many
    ? `<div class="tabs" role="tablist" aria-label="选择显示器">${view.monitors
        .map(
          (m) =>
            `<button class="tab" role="tab" data-tab="${esc(m.key)}" aria-selected="${m.key === monitor.key}">${esc(m.label)}</button>`,
        )
        .join("")}</div>`
    : "";
  const title = many ? "" : `<span class="title">${esc(monitor.label)}</span>`;
  const sub = monitor.currentName ? `正在显示 ${esc(monitor.currentName)}` : "读不到当前输入";
  const error = failure ?? monitor.error;
  const toggleTile = `<button class="tile" data-toggle>
    <span class="top"><span class="glyph">${icon("toggle", 20)}</span></span>
    <span class="labels"><span class="name">来回切换</span><span class="port">${esc(prettyAccelerator(view.toggleHotkey) || "未设置快捷键")}</span></span>
  </button>`;
  panel.innerHTML = `${tabs}
    <div class="head"><div class="titles">${title}<span class="sub">${sub}</span></div>${settingsButton()}</div>
    ${error ? `<div class="error" role="alert">${esc(error)}</div>` : ""}
    <div class="grid">${monitor.ports.map((p) => tile(monitor, p)).join("")}${toggleTile}</div>`;
  afterRender();
}

function afterRender(): void {
  panel.querySelectorAll<HTMLButtonElement>("[data-tab]").forEach((b) =>
    b.addEventListener("click", () => {
      selectedKey = b.dataset.tab ?? null;
      render();
    }),
  );
  panel.querySelectorAll<HTMLButtonElement>("[data-value]").forEach((b) =>
    b.addEventListener("click", () => {
      void api.switchInput(b.dataset.key!, Number(b.dataset.value));
    }),
  );
  panel.querySelector<HTMLButtonElement>("[data-toggle]")?.addEventListener("click", () => void api.toggle());
  panel.querySelector<HTMLButtonElement>("[data-settings]")?.addEventListener("click", () => void api.openSettings());
  requestAnimationFrame(() => void api.fitFlyout(Math.ceil(panel.getBoundingClientRect().height)));
}

document.addEventListener("keydown", (e) => {
  if (e.key === "Escape") void api.hideFlyout();
});

void onView((v) => {
  view = v;
  render();
});

void onSwitchFailed((f) => {
  failure = f.reason;
  render();
  window.clearTimeout(failureTimer);
  failureTimer = window.setTimeout(() => {
    failure = null;
    render();
  }, 4000);
});

void api.getView().then((v) => {
  view = v;
  render();
});
```

- [ ] **Step 7: 类型检查和构建**

Run: `npm run build`
Expected: 没有 TypeScript 错误，生成 `dist/`。

- [ ] **Step 8: Commit**

```bash
git add src flyout.html
git commit -m "feat: tray panel with input tiles"
```

---

### Task 14: 屏幕提示页面

**Files:**
- Modify: `src/osd/main.ts`、`src/osd/osd.css`

- [ ] **Step 1: 写 `src/osd/osd.css`**

```css
body {
  display: flex;
  justify-content: center;
  padding-top: 4px;
}

.osd {
  width: 252px;
  padding: 22px 20px 20px;
  border-radius: 24px;
  background: rgba(32, 33, 36, 0.94);
  border: 1px solid var(--border);
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
  opacity: 0;
}

.osd.show {
  animation: osd 1.5s ease forwards;
}

@keyframes osd {
  0% {
    opacity: 0;
    transform: scale(0.96);
  }
  10% {
    opacity: 1;
    transform: scale(1);
  }
  80% {
    opacity: 1;
  }
  100% {
    opacity: 0;
  }
}

.osd .glyph {
  width: 60px;
  height: 60px;
  border-radius: 18px;
  background: var(--accent);
  color: #fff;
  display: flex;
  align-items: center;
  justify-content: center;
}

.osd .title,
.osd .detail {
  font-size: 12px;
  color: var(--muted);
}

.osd .name {
  font-size: 20px;
  font-weight: 600;
  text-align: center;
  max-width: 100%;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
```

- [ ] **Step 2: 写 `src/osd/main.ts`**

```ts
import { listen } from "@tauri-apps/api/event";
import { esc } from "../shared/api";
import { icon } from "../shared/icons";

interface OsdPayload {
  title: string;
  name: string;
  detail: string;
  icon: string;
}

const root = document.getElementById("osd") as HTMLElement;

void listen<OsdPayload>("osd-show", ({ payload }) => {
  root.innerHTML = `<span class="glyph">${icon(payload.icon, 32)}</span>
    <span class="title">${esc(payload.title)}</span>
    <span class="name">${esc(payload.name)}</span>
    <span class="detail">${esc(payload.detail)}</span>`;
  // Restart the fade animation.
  root.classList.remove("show");
  void root.offsetWidth;
  root.classList.add("show");
});
```

- [ ] **Step 3: 类型检查和构建**

Run: `npm run build`
Expected: 没有错误。

- [ ] **Step 4: Commit**

```bash
git add src/osd
git commit -m "feat: on-screen switch prompt"
```

---

### Task 15: 设置窗口

**Files:**
- Modify: `src/settings/main.ts`、`src/settings/settings.css`

- [ ] **Step 1: 写 `src/settings/settings.css`**

```css
html,
body {
  background: #1e1f22;
}

body {
  padding: 20px 22px 28px;
}

h1 {
  margin: 0 0 14px;
  font-size: 20px;
  font-weight: 600;
}

.section-title {
  font-size: 12px;
  color: var(--muted);
  margin: 18px 0 8px;
}

.row {
  display: flex;
  align-items: center;
  gap: 10px;
  font-size: 13px;
  margin-bottom: 10px;
}

.row > label {
  color: var(--muted);
  white-space: nowrap;
}

.grow {
  flex: 1;
}

select,
input[type="text"] {
  font: inherit;
  font-size: 13px;
  color: var(--text);
  background: #1b1c1f;
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 8px;
  height: 30px;
  padding: 0 10px;
  min-width: 0;
}

.port {
  display: flex;
  gap: 12px;
  align-items: center;
  padding: 10px 12px;
  border-radius: 12px;
  background: #26272b;
  margin-bottom: 10px;
}

.port.unnamed {
  background: transparent;
  border: 1px dashed rgba(255, 255, 255, 0.18);
}

.port .glyph {
  width: 40px;
  height: 40px;
  border-radius: 10px;
  background: #2f3a52;
  color: #a9c4ff;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}

.port.unnamed .glyph {
  background: #26272b;
  color: var(--faint);
}

.fields {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.fields label {
  font-size: 11px;
  color: var(--muted);
}

.hk {
  height: 30px;
  padding: 0 10px;
  border-radius: 8px;
  border: 1px solid rgba(255, 255, 255, 0.12);
  background: #1b1c1f;
  font-size: 12px;
  white-space: nowrap;
}

.hk.empty {
  border-style: dashed;
  color: var(--muted);
  background: transparent;
}

.hk.rec {
  border-color: var(--accent);
  color: #fff;
}

.hk.bad {
  border-color: var(--danger);
}

.err {
  font-size: 11px;
  color: var(--danger);
}

.card {
  background: #26272b;
  border-radius: 12px;
  padding: 2px 12px;
}

.switch-row {
  display: flex;
  align-items: center;
  gap: 10px;
  min-height: 40px;
  font-size: 13px;
  border-bottom: 1px solid rgba(255, 255, 255, 0.06);
}

.switch-row:last-child {
  border-bottom: 0;
}

.switch-row span {
  flex: 1;
}

input[type="checkbox"] {
  appearance: none;
  width: 36px;
  height: 20px;
  border-radius: 10px;
  background: #45474d;
  position: relative;
  margin: 0;
  cursor: pointer;
  flex-shrink: 0;
}

input[type="checkbox"]::after {
  content: "";
  position: absolute;
  top: 3px;
  left: 3px;
  width: 14px;
  height: 14px;
  border-radius: 7px;
  background: #fff;
  transition: left 0.15s;
}

input[type="checkbox"]:checked {
  background: var(--accent);
}

input[type="checkbox"]:checked::after {
  left: 19px;
}

input[type="checkbox"]:focus-visible,
button:focus-visible,
select:focus-visible,
input[type="text"]:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: 2px;
}

.note {
  font-size: 12px;
  color: var(--muted);
  margin: 6px 0 0;
  line-height: 1.6;
}
```

- [ ] **Step 2: 写 `src/settings/main.ts`**

```ts
import { api, esc, onView, type MonitorView, type OptionName, type PortView, type View } from "../shared/api";
import { acceleratorFromEvent, prettyAccelerator } from "../shared/hotkey";
import { icon } from "../shared/icons";

type Target = { kind: "port"; key: string; value: number } | { kind: "toggle" };

const root = document.getElementById("app") as HTMLElement;
let view: View | null = null;
let selectedKey: string | null = null;
let recording: { target: Target; errorId: string } | null = null;
const errors = new Map<string, string>();
let renderPending = false;

const sameTarget = (a: Target, b: Target) =>
  a.kind === b.kind && (a.kind === "toggle" || (b.kind === "port" && a.key === b.key && a.value === b.value));

function selectedMonitor(): MonitorView | undefined {
  return view?.monitors.find((m) => m.key === selectedKey) ?? view?.monitors[0];
}

function hotkeyButton(target: Target, accelerator: string | null, errorId: string): string {
  const isRecording = recording !== null && sameTarget(recording.target, target);
  const refused = accelerator !== null && (view?.failedHotkeys ?? []).includes(accelerator);
  const text = isRecording ? "按下组合键…" : accelerator ? prettyAccelerator(accelerator) : "录制";
  const classes = ["hk", accelerator ? "" : "empty", isRecording ? "rec" : "", errors.has(errorId) || refused ? "bad" : ""]
    .join(" ")
    .trim();
  const title = refused ? "已被其他程序占用" : "点一下，然后按下组合键；Backspace 清除，Esc 取消";
  return `<button class="${classes}" data-hk='${esc(JSON.stringify(target))}' data-err="${errorId}" title="${title}">${esc(text)}</button>`;
}

function portCard(monitor: MonitorView, port: PortView): string {
  const id = `port-${port.value}`;
  const unnamed = !port.name && !port.isLocal;
  let label = esc(port.portLabel);
  if (port.isLocal) label += monitor.localOverride === null ? " · 本机，自动识别" : " · 本机";
  else if (!port.name) label += " · 还没起名字";
  const error = errors.get(id);
  const placeholder = port.isLocal ? esc(view!.computerName) : "起个名字";
  return `<div class="port${unnamed ? " unnamed" : ""}">
    <span class="glyph">${icon(port.icon)}</span>
    <div class="fields">
      <label for="${id}">${label}</label>
      <input type="text" id="${id}" data-name="${port.value}" value="${esc(port.name ?? "")}" placeholder="${placeholder}" maxlength="40" />
      ${error ? `<span class="err" role="alert">${esc(error)}</span>` : ""}
    </div>
    ${hotkeyButton({ kind: "port", key: monitor.key, value: port.value }, port.hotkey, id)}
  </div>`;
}

function portOptions(monitor: MonitorView, selected: number | null): string {
  return monitor.ports
    .map((p) => `<option value="${p.value}"${p.value === selected ? " selected" : ""}>${esc(p.displayName)}（${esc(p.portLabel)}）</option>`)
    .join("");
}

function monitorSection(v: View, monitor: MonitorView): string {
  const picker =
    v.monitors.length > 1
      ? `<select id="monitor" class="grow">${v.monitors
          .map((m) => `<option value="${esc(m.key)}"${m.key === monitor.key ? " selected" : ""}>${esc(m.label)}</option>`)
          .join("")}</select>`
      : `<span class="grow">${esc(monitor.label)}</span>`;
  const pair = monitor.togglePair;
  const toggleError = errors.get("toggle");
  const pairError = errors.get("pair");
  return `
    <div class="row"><label for="monitor">显示器</label>${picker}<span class="note">共 ${v.monitors.length} 台</span></div>
    <div class="section-title">接口与快捷键</div>
    ${monitor.ports.map((p) => portCard(monitor, p)).join("")}
    <div class="section-title">来回切换</div>
    <div class="row">
      <label for="pair-a">在</label><select id="pair-a" class="grow">${portOptions(monitor, pair?.[0] ?? null)}</select>
      <label for="pair-b">和</label><select id="pair-b" class="grow">${portOptions(monitor, pair?.[1] ?? null)}</select>
      ${hotkeyButton({ kind: "toggle" }, v.toggleHotkey, "toggle")}
    </div>
    ${toggleError ? `<p class="err" role="alert">${esc(toggleError)}</p>` : ""}
    ${pairError ? `<p class="err" role="alert">${esc(pairError)}</p>` : ""}
    <div class="row">
      <label for="local">本机接在</label>
      <select id="local" class="grow"><option value="">自动识别</option>${monitor.ports
        .map((p) => `<option value="${p.value}"${monitor.localOverride === p.value ? " selected" : ""}>${esc(p.portLabel)}</option>`)
        .join("")}</select>
    </div>
    <div class="section-title">高级</div>
    <div class="card">
      <label class="switch-row"><span>自动读取这台显示器的接口列表</span><input type="checkbox" data-caps${monitor.readCapabilities ? " checked" : ""} /></label>
    </div>
    <p class="note">极少数显示器在读取接口列表时会让 Windows 崩溃。关掉后按标准接口显示；如果上次读取时崩溃了，这里会自动关掉。</p>`;
}

function switchRow(name: OptionName, text: string, on: boolean): string {
  return `<label class="switch-row"><span>${text}</span><input type="checkbox" data-opt="${name}"${on ? " checked" : ""} /></label>`;
}

function render(): void {
  if (!view) return;
  // Don't wipe out what the user is typing; render again when the field loses focus.
  const active = document.activeElement;
  if (active instanceof HTMLInputElement && active.type === "text") {
    renderPending = true;
    return;
  }
  renderPending = false;
  const monitor = selectedMonitor();
  selectedKey = monitor?.key ?? null;
  root.innerHTML = `
    <h1>设置</h1>
    ${monitor ? monitorSection(view, monitor) : `<p class="note">没找到能控制的显示器。确认显示器菜单里 DDC/CI 已开启。</p>`}
    <div class="section-title">常规</div>
    <div class="card">
      ${switchRow("showOsd", "切换时显示屏幕提示", view.showOsd)}
      ${switchRow("autostart", "开机时启动", view.autostart)}
    </div>`;
  bind();
}

async function run(errorId: string, action: () => Promise<void>): Promise<void> {
  try {
    await action();
    errors.delete(errorId);
  } catch (e) {
    errors.set(errorId, String(e));
  }
  render();
}

function bind(): void {
  const monitor = selectedMonitor();

  root.querySelector<HTMLSelectElement>("select#monitor")?.addEventListener("change", (e) => {
    selectedKey = (e.target as HTMLSelectElement).value;
    render();
  });

  root.querySelectorAll<HTMLInputElement>("input[data-name]").forEach((input) => {
    input.addEventListener("change", () => {
      if (!monitor) return;
      const value = Number(input.dataset.name);
      void run(`port-${value}`, () => api.setPortName(monitor.key, value, input.value.trim() || null));
    });
    input.addEventListener("keydown", (e) => {
      if (e.key === "Enter") input.blur();
    });
    input.addEventListener("blur", () => {
      if (renderPending) window.setTimeout(render, 0);
    });
  });

  root.querySelectorAll<HTMLButtonElement>("button[data-hk]").forEach((button) =>
    button.addEventListener("click", () => {
      recording = { target: JSON.parse(button.dataset.hk!) as Target, errorId: button.dataset.err! };
      render();
    }),
  );

  const pairA = root.querySelector<HTMLSelectElement>("#pair-a");
  const pairB = root.querySelector<HTMLSelectElement>("#pair-b");
  const onPair = () => {
    if (!monitor || !pairA || !pairB) return;
    void run("pair", () => api.setTogglePair(monitor.key, [Number(pairA.value), Number(pairB.value)]));
  };
  pairA?.addEventListener("change", onPair);
  pairB?.addEventListener("change", onPair);

  root.querySelector<HTMLSelectElement>("#local")?.addEventListener("change", (e) => {
    if (!monitor) return;
    const v = (e.target as HTMLSelectElement).value;
    void run("local", () => api.setLocalOverride(monitor.key, v === "" ? null : Number(v)));
  });

  root.querySelector<HTMLInputElement>("input[data-caps]")?.addEventListener("change", (e) => {
    if (!monitor) return;
    void run("caps", () => api.setReadCapabilities(monitor.key, (e.target as HTMLInputElement).checked));
  });

  root.querySelectorAll<HTMLInputElement>("input[data-opt]").forEach((input) =>
    input.addEventListener("change", () => {
      void run("opt", () => api.setOption(input.dataset.opt as OptionName, input.checked));
    }),
  );
}

// Hotkey recording: the next key combination pressed anywhere in the window.
document.addEventListener(
  "keydown",
  (e) => {
    if (!recording) return;
    e.preventDefault();
    e.stopPropagation();
    const { target, errorId } = recording;
    if (e.key === "Escape") {
      recording = null;
      render();
      return;
    }
    let accelerator: string | null;
    if (e.key === "Backspace" || e.key === "Delete") {
      accelerator = null;
    } else {
      accelerator = acceleratorFromEvent(e);
      if (accelerator === null) return; // still holding only modifiers
    }
    recording = null;
    void run(errorId, () =>
      target.kind === "toggle" ? api.setToggleHotkey(accelerator) : api.setPortHotkey(target.key, target.value, accelerator),
    );
  },
  true,
);

void onView((v) => {
  view = v;
  render();
});

void api.getView().then((v) => {
  view = v;
  render();
});
```

- [ ] **Step 3: 类型检查和构建**

Run: `npm run build`
Expected: 没有错误。

- [ ] **Step 4: Commit**

```bash
git add src/settings
git commit -m "feat: settings window with names, hotkeys, toggle pair and options"
```

---

### Task 16: 真机验证、打包、说明文档

**Files:**
- Create: `README.md`

- [ ] **Step 1: 跑全部自动测试**

```bash
cargo test --manifest-path src-tauri/Cargo.toml
npm run build
```

Expected: Rust 测试全部通过（1 个 ignored）；前端构建无错误。

- [ ] **Step 2: 开发模式启动，逐项检查**

Run: `npm run tauri dev`

逐项确认（任何一项不符合就先修再继续）：

1. 任务栏右下角出现托盘图标；悬停显示"Dell U2723QE · <本机名>（DP）"。
2. 左键托盘图标：面板贴着托盘弹出；只有 Dell 一张卡片区，DP 磁贴带"本机"标签并高亮；显卡诱骗器不出现。
3. 点面板外任意处，面板收起；再左键托盘图标，面板重新打开（不会闪一下又关掉）。
4. 打开设置，把 USB-C 命名为"公司笔记本"：面板和右键菜单里同步出现这个名字。
5. 笔记本开着并接在 USB-C 上时，点"公司笔记本"磁贴：屏幕中央出现"正在切换到 公司笔记本"，约 0.6 秒后显示器切到笔记本。用显示器按键切回主机。
6. 按 Ctrl+Alt+M 切到笔记本，再在主机上通过 ToDesk 或显示器按键回到主机，按 Ctrl+Alt+M：切回 DP，屏幕出现"已切换到 <本机名>"。
7. 在设置里给 DP 录制 Ctrl+Alt+1；再给 USB-C 也录 Ctrl+Alt+1：提示"这个快捷键已经给别的接口用了"。
8. 关闭设置窗口，托盘图标仍在；右键 → 退出，程序结束。
9. 查看 `%APPDATA%\MonitorSwitch\config.json`：有 Dell 的条目，`cachedCapabilities` 已填，`capsReadPending` 为 false。

- [ ] **Step 3: 打包发布版**

Run: `npm run tauri build`
Expected: 生成 `src-tauri/target/release/bundle/nsis/MonitorSwitch_0.1.0_x64-setup.exe`，大小小于 10 MB（规格 1 节）。第一次打包时 Tauri 会自动下载 NSIS 工具。

安装后确认：

1. 开始菜单出现 MonitorSwitch。
2. 程序已在运行时，再从开始菜单打开一次：不会出现第二个托盘图标，只是弹出已运行那份的面板（单实例，规格 6.7）。
3. 设置里"开机时启动"打开时，注销再登录后托盘图标自动出现。

- [ ] **Step 4: 写 `README.md`**

````markdown
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
````

- [ ] **Step 5: Commit**

```bash
git add README.md
git commit -m "docs: README with usage and development commands"
```

---

## 规格覆盖自查

| 规格要求 | 任务 |
|---|---|
| 3.1 托盘图标、悬停提示、左键面板、右键菜单（单台平铺、多台子菜单） | 11、12 |
| 3.2 面板：多显示器标签、"正在显示"、磁贴、本机标签、虚线未命名、来回切换磁贴、打开即刷新、无响应提示 | 9、11、13 |
| 3.3 OSD：内容、穿透、1.5 秒、可关闭 | 12、14、15 |
| 3.5 设置：显示器下拉、接口卡片、名称、快捷键录制（Esc/Backspace）、来回切换接口对和快捷键、OSD 开关、自启、高级里的读取接口列表开关、即时保存 | 12、15 |
| 3.5 局域网开关 | 计划 3（设置项字段已在 Task 5 预留） |
| 5.1 模块划分 | 2–12 |
| 5.2 切换：先 OSD 再切、重试 3 次、1.5 秒读回、本机时 OSD；轮询 4 秒、外部变化；另一台电脑上的提示；来回切换作用于所有配置了接口对的显示器；接口快捷键只作用于所属显示器 | 7、12 |
| 5.2 新设备提示、锁屏暂停轮询 | 计划 2 |
| 5.3 本机接口识别 + 手动指定 | 3、7、15 |
| 5.4 设置文件位置、格式、原子写、损坏备份、version | 5、12 |
| 6.1 DDC 失败不弹窗 | 7、13 |
| 6.2 快捷键冲突（本程序内、其他程序） | 10、12、15 |
| 6.3 显示器断开、重连 | 7（每 5 次轮询重新枚举，打开面板时也重新枚举） |
| 6.4 读接口列表的崩溃保护 | 5、11、12、15 |
| 6.6 设置文件损坏 | 5 |
| 6.7 单实例 | 12 |
| 7 单元测试、真机测试 | 2–12、16 |
| 8 Windows 安装包 | 16 |
