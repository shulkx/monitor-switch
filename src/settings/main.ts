import { api, esc, onHotkeyRecorded, onView, type MonitorView, type OptionName, type PortView, type View } from "../shared/api";
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

const REFUSED = "被其他程序占用了，这个快捷键现在不起作用。点右边重新录一个";

/** True when the OS refused to register `accelerator` (another program got it first). */
function refused(accelerator: string | null): boolean {
  return accelerator !== null && (view?.failedHotkeys ?? []).includes(accelerator);
}

/** The error to show for a hotkey: the last attempt's, else whether the saved one is taken. */
function hotkeyError(errorId: string, accelerator: string | null): string | undefined {
  if (recording !== null && recording.errorId === errorId) return undefined;
  return errors.get(errorId) ?? (refused(accelerator) ? REFUSED : undefined);
}

function hotkeyButton(target: Target, accelerator: string | null, errorId: string): string {
  const isRecording = recording !== null && sameTarget(recording.target, target);
  const text = isRecording ? "按下组合键…" : accelerator ? prettyAccelerator(accelerator) : "录制";
  const bad = !isRecording && hotkeyError(errorId, accelerator) !== undefined;
  const classes = ["hk", accelerator ? "" : "empty", isRecording ? "rec" : "", bad ? "bad" : ""].join(" ").trim();
  const title = "点一下，然后按下组合键；Backspace 清除，Esc 取消";
  return `<button class="${classes}" data-hk='${esc(JSON.stringify(target))}' data-err="${errorId}" title="${title}">${esc(text)}</button>`;
}

function portCard(monitor: MonitorView, port: PortView): string {
  const id = `port-${port.value}`;
  const unnamed = !port.name && !port.isLocal;
  let label = esc(port.portLabel);
  if (port.isLocal) label += monitor.localOverride === null ? " · 本机，自动识别" : " · 本机";
  else if (!port.name) label += " · 还没起名字";
  const error = errors.get(id) ?? hotkeyError(id, port.hotkey);
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
  const placeholder = `<option value="" disabled${selected === null ? " selected" : ""}>选择接口</option>`;
  return (
    placeholder +
    monitor.ports
      .map((p) => `<option value="${p.value}"${p.value === selected ? " selected" : ""}>${esc(p.displayName)}（${esc(p.portLabel)}）</option>`)
      .join("")
  );
}

function monitorSection(v: View, monitor: MonitorView): string {
  const picker =
    v.monitors.length > 1
      ? `<select id="monitor" class="grow">${v.monitors
          .map((m) => `<option value="${esc(m.key)}"${m.key === monitor.key ? " selected" : ""}>${esc(m.label)}</option>`)
          .join("")}</select>`
      : `<span class="grow">${esc(monitor.label)}</span>`;
  const pair = monitor.togglePair;
  const toggleError = hotkeyError("toggle", v.toggleHotkey);
  const pairError = errors.get("pair");
  return `
    <div class="row"><label for="monitor">显示器</label>${picker}<span class="note">共 ${v.monitors.length} 台</span></div>
    <div class="section-title">接口与快捷键</div>
    ${monitor.ports.map((p) => portCard(monitor, p)).join("")}
    <div class="section-title">来回切换</div>
    <div class="row">
      <select id="pair-a" class="grow" aria-label="第一个接口">${portOptions(monitor, pair?.[0] ?? null)}</select>
      <span class="swap" aria-hidden="true">⇄</span>
      <select id="pair-b" class="grow" aria-label="第二个接口">${portOptions(monitor, pair?.[1] ?? null)}</select>
    </div>
    <div class="row">
      <span class="row-label">快捷键</span>
      <span class="grow hint">按一下，在这两个接口之间来回切</span>
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

/** Version, update status and the button that fits it. */
function aboutSection(v: View): string {
  const u = v.update;
  const check = `<button class="btn" data-check>检查更新</button>`;
  let status = "";
  let actions = check;
  switch (u.state) {
    case "checking":
      status = "正在检查…";
      actions = "";
      break;
    case "upToDate":
      status = "已是最新版本";
      break;
    case "available":
      status = `发现新版本 ${esc(u.version)}`;
      actions = `<button class="btn primary" data-install>更新到 ${esc(u.version)}</button>`;
      break;
    case "downloading":
      status = `正在下载 ${esc(u.version)}${u.percent === null ? "…" : ` · ${u.percent}%`}`;
      actions = "";
      break;
    case "failed":
      status = `<span class="err">${esc(u.message)}</span>`;
      actions = `${check}<button class="btn" data-release>去下载页</button>`;
      break;
  }
  const notes = u.state === "available" && u.notes ? `<p class="note release-notes">${esc(u.notes)}</p>` : "";
  return `<div class="section-title">关于</div>
    <div class="card">
      <div class="about-row">
        <div class="about-text"><span>MonitorSwitch ${esc(v.appVersion)}</span>${status ? `<span class="note">${status}</span>` : ""}</div>
        ${actions}
      </div>
      ${switchRow("checkUpdates", "自动检查更新", v.checkUpdates)}
    </div>${notes}`;
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
    </div>
    ${aboutSection(view)}`;
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
    button.addEventListener("click", () => beginRecording(JSON.parse(button.dataset.hk!) as Target, button.dataset.err!)),
  );

  const pairA = root.querySelector<HTMLSelectElement>("#pair-a");
  const pairB = root.querySelector<HTMLSelectElement>("#pair-b");
  const onPair = () => {
    if (!monitor || !pairA || !pairB || pairA.value === "" || pairB.value === "") return;
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

  root.querySelector<HTMLButtonElement>("[data-check]")?.addEventListener("click", () => void api.checkForUpdates());
  root.querySelector<HTMLButtonElement>("[data-install]")?.addEventListener("click", () => void api.installUpdate());
  root.querySelector<HTMLButtonElement>("[data-release]")?.addEventListener("click", () => void api.openReleasePage());

  root.querySelectorAll<HTMLInputElement>("input[data-opt]").forEach((input) =>
    input.addEventListener("change", () => {
      void run("opt", () => api.setOption(input.dataset.opt as OptionName, input.checked));
    }),
  );
}

/**
 * Our own hotkeys go off while recording, so pressing one records it (and shows the conflict)
 * instead of switching the monitor. On Windows the backend also catches combinations another
 * program owns, which would otherwise never reach this page, and sends them as "hotkey-recorded".
 */
function beginRecording(target: Target, errorId: string): void {
  recording = { target, errorId };
  errors.delete(errorId);
  void api.startHotkeyRecording();
  render();
}

function stopRecording(): void {
  recording = null;
  void api.stopHotkeyRecording();
  render();
}

/** Save the recorded combination (null clears it). Taken or clashing ones come back as errors. */
function finishRecording(accelerator: string | null): void {
  if (!recording) return;
  const { target, errorId } = recording;
  recording = null;
  void run(errorId, async () => {
    try {
      await (target.kind === "toggle"
        ? api.setToggleHotkey(accelerator)
        : api.setPortHotkey(target.key, target.value, accelerator));
    } finally {
      // A rejected hotkey returns before the backend re-registers anything.
      await api.stopHotkeyRecording();
    }
  });
}

// Keys that reach the page while recording: Esc, Backspace/Delete, and (outside Windows)
// the combination itself.
document.addEventListener(
  "keydown",
  (e) => {
    if (!recording) return;
    e.preventDefault();
    e.stopPropagation();
    if (e.key === "Escape") {
      stopRecording();
    } else if (e.key === "Backspace" || e.key === "Delete") {
      finishRecording(null);
    } else {
      const accelerator = acceleratorFromEvent(e);
      if (accelerator !== null) finishRecording(accelerator); // null: still holding only modifiers
    }
  },
  true,
);

void onHotkeyRecorded((accelerator) => finishRecording(accelerator));

// Leaving the window while recording cancels it, so hotkeys never stay switched off.
window.addEventListener("blur", () => {
  if (recording) stopRecording();
});

void onView((v) => {
  view = v;
  render();
});

void api.getView().then((v) => {
  view = v;
  render();
});
