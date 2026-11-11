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
