# 面板贴着托盘图标弹出 + 更新提醒

日期：2026-11-12
状态：已确认（方案在对话里逐项确认过）

## 1. 背景

用户反馈两件事：

1. 左键点托盘图标，面板没有"贴着图标"出现；图标收在溢出菜单里时，点完"菜单就消失了"。
2. 需要自动更新。

### 实测结论

在用户机器上（Win10，3840×2160，150% 缩放，任务栏在底部）分别点溢出菜单里的 MonitorSwitch 和 OneDrive：

- **MonitorSwitch 0.1.1**：面板窗口 (3156,1602)-(3684,2082)，水平对准点击点，但底边一律落在任务栏上方 18 px（12 逻辑 px）。溢出菜单 (3300,1956)-(3492,2100) 被面板整个盖住，看起来像消失了。按 Esc 关掉面板后，溢出菜单仍然开着。
- **OneDrive**：窗口 (3139,1062)-(3679,2022)，水平对准图标，底边停在被点图标上方，溢出菜单仍露在外面。

所以问题不在 Windows 收起了溢出菜单，而在面板用"鼠标点在哪"来定位，没考虑图标和溢出菜单占的位置。

## 2. 面板定位

### 规则

- 定位依据从"点击坐标"改为托盘事件自带的**图标矩形**（`TrayIconEvent::Click.rect`，物理像素）。
- **图标所在窗口**：取图标中心点所在的顶层窗口（`WindowFromPoint` → `GetAncestor(GA_ROOT)`）。
  - 类名是 `Shell_TrayWnd` 或 `Shell_SecondaryTrayWnd`：图标在任务栏上，没有"宿主"。
  - 其他（Win10 的 `NotifyIconOverflowWindow`、Win11 的 XAML 溢出窗口等）：图标在溢出菜单里，宿主就是这个窗口的矩形。
  - 不靠类名识别溢出菜单，Win10 和 Win11 都适用。
- **任务栏在哪一边**：比较显示器范围和工作区，工作区在哪一边缩进，任务栏就在哪一边。任务栏自动隐藏（工作区等于显示器范围）时，取离图标中心最近的那条屏幕边。
- **放置**（`avoid` = 宿主矩形，没有宿主时就是图标矩形）：
  - 任务栏在下：面板底边 = min(`avoid`.top, 工作区底边) − 间隙，水平居中对准图标。
  - 在上、左、右三边的规则对称。
  - 沿任务栏方向限制在工作区内，两端各留边距。
  - 垂直于任务栏方向不得超出工作区：空间不够时宁可盖住溢出菜单，也不让面板出屏。
- **间隙** 8 逻辑 px（原来 12），**边距** 12 逻辑 px。
- 第二次启动程序（单实例转发）时没有图标矩形，仍放在主屏工作区右下角，行为不变。

### 结构

- `app/flyout.rs`：
  - 纯函数 `place(icon, host, monitor, work, size, gap, margin) -> (x, y)` 和 `taskbar_edge(monitor, work, icon) -> Edge`，带单元测试。
  - `FlyoutState.anchor` 改存 `Anchor { icon: Rect, host: Option<Rect> }`。
- `platform::tray_icon_host(icon: Rect) -> Option<Rect>`：Windows 实现在 `platform/win`，其他平台返回 `None`。
- 托盘点击时就查出宿主并存下：之后面板高度变化重新定位时，溢出菜单可能已经关了。

## 3. 更新提醒（"只提醒"）

### 行为

- **检查**：
  - 启动 30 秒后查一次，之后每 6 小时查一次 `https://github.com/shulkx/monitor-switch/releases/latest/download/latest.json`。
  - 设置里的"自动检查更新"开关（默认开）关掉后，不再定时检查。
  - "检查更新"按钮随时可用。
- **只提醒**：发现新版本时只改界面状态，不下载，不弹窗。提示出现在三处：
  1. 托盘面板底部一条：`新版本 0.1.3` + `更新` 按钮。
  2. 右键菜单在"设置"上方加一项：`更新到 0.1.3`。
  3. 设置页新增"关于"区：当前版本、状态、更新说明、按钮。
- **点"更新"后**：
  1. 下载并显示进度（面板和设置里显示百分比，右键菜单显示"正在下载更新…"）。
  2. 插件用内置公钥校验签名，校验不过就不安装。
  3. 安装前调用 `AppHandle::cleanup_before_exit()` 移除托盘图标，避免 `process::exit` 留下残影。
  4. 安装包以 passive 模式运行，参数 `/P /UPDATE /R`：只显示进度条，装完自动重新打开程序。
  5. 程序退出。设置文件不受影响。
- **失败处理**：
  - 后台检查失败时，不改变当前显示的状态，也不打扰用户。
  - 手动检查失败、下载或安装失败时，显示原因，并提供"去下载页"链接，打开 GitHub Releases 页面手动下载。

### 状态

```rust
#[serde(tag = "state")]
enum UpdateStatus {
    Idle,
    Checking,
    UpToDate,
    Available { version, notes },
    Downloading { version, percent },
    Failed { message, version },
}
```

- `View` 新增三个字段：
  - `appVersion`（当前版本）；
  - `update: UpdateStatus`；
  - `checkUpdates: bool`。
- 找到的 `tauri_plugin_updater::Update` 对象存在 `AppState` 里，点"更新"时直接用。
- 下载进度至少涨 5% 才更新一次状态，以免频繁重建右键菜单。
- 状态转换写成纯函数，便于单元测试。例如，后台检查失败时保留原状态，手动检查失败时变为 `Failed`。

### 设置项

`Config.check_updates: bool`，默认 `true`。旧设置文件里没有这个字段时按默认值处理（`#[serde(default)]` 已覆盖）。通过 `set_option("checkUpdates", …)` 修改。

### 签名与发布

- **密钥**：用 `tauri signer generate` 生成一对密钥。
  - 私钥放在 `%USERPROFILE%\.tauri\monitor-switch.key`，不设密码，不进仓库。
  - 公钥写进 `tauri.conf.json` 的 `plugins.updater.pubkey`。
  - 私钥丢失后，已安装的程序无法再收到更新，需要手动重装一个换了新公钥的版本。
- **构建开关**：`tauri.conf.json` 里不打开 `createUpdaterArtifacts`。这样没有私钥的人照样能用 `npm run tauri build` 构建；只有发布脚本用 `--config` 临时打开它。
- **`npm run release`**（`scripts/release.mjs`）：
  1. 核对 `tauri.conf.json`、`Cargo.toml`、`package.json` 三处版本号一致。
  2. 从环境变量或密钥文件读出私钥，带签名构建。
  3. 用公钥校验 `.sig`（`scripts/minisign.mjs`，算法与插件相同：Ed25519 + BLAKE2b-512 预哈希，同时校验 trusted comment 的全局签名）。
  4. 生成 `latest.json`：`version`、`notes`、`pub_date`，以及 `platforms.windows-x86_64` 下的 `signature` 和 `url`。
  5. 输出安装包的 SHA-256。
  6. 只有带 `--publish` 时才执行 `gh release create`，上传安装包和 `latest.json`。
- **过渡**：0.1.1 没有更新功能，0.1.2 需要手动安装一次。

## 4. 测试

- **Rust 单元测试**：
  - 定位：任务栏在下、上、左、右；图标在溢出菜单里；贴近屏幕边时的限位；自动隐藏的任务栏；面板比可用空间还高。
  - 更新：状态转换和下载百分比。
  - 设置：旧设置文件缺少 `checkUpdates` 字段时读出 `true`。
- **Node 测试**：`node --test scripts/`，校验一个用临时密钥签过名的样例。签名正确时通过；文件被改、签名被改、换了公钥时都应失败。
- **类型检查**：`npm run build`（`tsc` + vite）。
- **实机测试**：用临时的程序标识运行开发版（`--config` 改 `identifier`），不影响用户正在运行的那份。
  - 在溢出菜单和任务栏两种位置点图标，截图核对面板位置。
  - 用 `--config` 把更新地址指向本地服务器上的假 `latest.json`，确认三处提示、手动检查、"已是最新"、失败提示都正确。
- **测不到的部分**：真正的"下载 → 安装 → 重开"会结束用户正在运行的程序，开发时不在用户机器上执行。发布 0.1.3 时，由已安装 0.1.2 的电脑走第一遍。

## 5. 不做

- 静默或自动下载安装（用户选了"只提醒"）。
- 系统通知弹窗、托盘图标角标。
- "跳过这个版本"。
- 自动构建发布（CI）。
