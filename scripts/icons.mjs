// Regenerates every app and tray icon from the two SVG sources in src-tauri/icons:
//   icon.svg        detailed version, used for 32 px and up
//   icon-small.svg  drawn on a 16 px grid, used for 16–24 px so the tray icon stays sharp
// Usage: npm run icons
import { execSync } from "node:child_process";
import { copyFileSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const ICONS = "src-tauri/icons";
const SMALL = [16, 20, 24];
const LARGE = [32, 40, 48, 64, 256];
const TRAY = [16, 20, 24, 32];

const tauriIcon = (args) => execSync(`npx tauri icon ${args}`, { stdio: "inherit" });

// The standard set (PNGs, .icns, Store logos) comes from the detailed source.
tauriIcon(`${ICONS}/icon.svg`);
rmSync(`${ICONS}/android`, { recursive: true, force: true });
rmSync(`${ICONS}/ios`, { recursive: true, force: true });

// Exact sizes from each source.
const tmp = mkdtempSync(join(tmpdir(), "monitorswitch-icons-"));
const sizes = (list) => list.map((s) => `-p ${s}`).join(" ");
tauriIcon(`${ICONS}/icon-small.svg -o "${tmp}/small" ${sizes(SMALL)}`);
tauriIcon(`${ICONS}/icon.svg -o "${tmp}/large" ${sizes(LARGE)}`);
const png = (size) => readFileSync(join(tmp, SMALL.includes(size) ? "small" : "large", `${size}x${size}.png`));

// icon.ico: every size, PNG-compressed like the one `tauri icon` writes.
const sizesInIco = [...SMALL, ...LARGE];
const images = sizesInIco.map(png);
const header = Buffer.alloc(6);
header.writeUInt16LE(1, 2); // type: icon
header.writeUInt16LE(images.length, 4);
let offset = 6 + 16 * images.length;
const entries = images.map((data, i) => {
  const size = sizesInIco[i];
  const entry = Buffer.alloc(16);
  entry.writeUInt8(size >= 256 ? 0 : size, 0);
  entry.writeUInt8(size >= 256 ? 0 : size, 1);
  entry.writeUInt16LE(1, 4); // colour planes
  entry.writeUInt16LE(32, 6); // bits per pixel
  entry.writeUInt32LE(data.length, 8);
  entry.writeUInt32LE(offset, 12);
  offset += data.length;
  return entry;
});
writeFileSync(`${ICONS}/icon.ico`, Buffer.concat([header, ...entries, ...images]));

// Tray icons: the app picks the one matching the system's small-icon size.
mkdirSync(`${ICONS}/tray`, { recursive: true });
for (const size of TRAY) copyFileSync(join(tmp, SMALL.includes(size) ? "small" : "large", `${size}x${size}.png`), `${ICONS}/tray/${size}.png`);

rmSync(tmp, { recursive: true, force: true });
console.log(`icon.ico: ${sizesInIco.join(", ")} px; tray: ${TRAY.join(", ")} px`);
