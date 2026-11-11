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
