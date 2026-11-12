// Build a signed installer and the update feed (latest.json) the app checks, verify the signature
// with the public key built into the app, and optionally publish both to GitHub Releases.
//
//   npm run release -- --notes notes.md            build and check only
//   npm run release -- --notes notes.md --publish  also create the GitHub release
//
// The private key comes from TAURI_SIGNING_PRIVATE_KEY, or from ~/.tauri/monitor-switch.key.

import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { homedir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { parseArgs } from "node:util";

import { verify } from "./minisign.mjs";

const REPO = "shulkx/monitor-switch";
const root = fileURLToPath(new URL("..", import.meta.url));
const { values: args } = parseArgs({ options: { notes: { type: "string" }, publish: { type: "boolean" } } });

function fail(message) {
  console.error(`release: ${message}`);
  process.exit(1);
}

function run(command, commandArgs, env = process.env) {
  // shell: npm and gh are .cmd scripts on Windows, so arguments with spaces need quotes.
  const quoted = commandArgs.map((a) => (/\s/.test(a) ? `"${a}"` : a));
  const result = spawnSync(command, quoted, { cwd: root, stdio: "inherit", shell: true, env });
  if (result.status !== 0) fail(`${command} ${quoted.join(" ")} failed`);
}

// 1. One version everywhere.
const conf = JSON.parse(readFileSync(join(root, "src-tauri/tauri.conf.json"), "utf8"));
const version = conf.version;
const cargo = readFileSync(join(root, "src-tauri/Cargo.toml"), "utf8").match(/^version = "(.+)"$/m)?.[1];
const pkg = JSON.parse(readFileSync(join(root, "package.json"), "utf8")).version;
if (cargo !== version || pkg !== version) fail(`versions differ: tauri.conf.json ${version}, Cargo.toml ${cargo}, package.json ${pkg}`);

if (!args.notes || !existsSync(args.notes)) fail("pass the release notes with --notes <file>");
const notes = readFileSync(args.notes, "utf8").trim();

// 2. Build with the signing key.
const keyFile = join(homedir(), ".tauri", "monitor-switch.key");
const key = process.env.TAURI_SIGNING_PRIVATE_KEY ?? (existsSync(keyFile) ? readFileSync(keyFile, "utf8").trim() : null);
if (!key) fail(`no signing key: set TAURI_SIGNING_PRIVATE_KEY or create ${keyFile}`);
run("npm", ["run", "tauri", "--", "build", "--config", "src-tauri/release.conf.json"], {
  ...process.env,
  TAURI_SIGNING_PRIVATE_KEY: key,
  TAURI_SIGNING_PRIVATE_KEY_PASSWORD: process.env.TAURI_SIGNING_PRIVATE_KEY_PASSWORD ?? "",
});

// 3. Verify the signature the way the installed app will.
const name = `${conf.productName}_${version}_x64-setup.exe`;
const dir = join(root, "src-tauri/target/release/bundle/nsis");
const installer = readFileSync(join(dir, name));
const signature = readFileSync(join(dir, `${name}.sig`), "utf8").trim();
try {
  verify(installer, signature, conf.plugins.updater.pubkey);
} catch (e) {
  fail(`the signature doesn't match the app's public key: ${e.message}`);
}

// 4. The update feed.
const feed = {
  version,
  notes,
  pub_date: new Date().toISOString(),
  platforms: {
    "windows-x86_64": {
      signature,
      url: `https://github.com/${REPO}/releases/download/v${version}/${name}`,
    },
  },
};
const feedPath = join(dir, "latest.json");
writeFileSync(feedPath, `${JSON.stringify(feed, null, 2)}\n`);

const sha256 = createHash("sha256").update(installer).digest("hex").toUpperCase();
console.log(`\n${name}\n  size    ${installer.length} bytes\n  SHA-256 ${sha256}\n  signature verified\n${feedPath}`);

// 5. Publish only when asked.
if (args.publish) {
  run("gh", ["release", "create", `v${version}`, join(dir, name), feedPath, "--repo", REPO, "--title", `MonitorSwitch ${version}`, "--notes-file", args.notes]);
}
