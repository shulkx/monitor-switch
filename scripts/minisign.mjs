// Check a minisign signature the way tauri-plugin-updater does, so a release is verified against
// the public key built into the app before it is uploaded. Tauri base64-encodes minisign's own
// text formats for both the key and the signature.

import { createHash, createPublicKey, verify as verifyEd25519 } from "node:crypto";

const ED25519_SPKI_PREFIX = Buffer.from("302a300506032b6570032100", "hex");
const TRUSTED = "trusted comment: ";

function lines(base64) {
  return Buffer.from(base64.trim(), "base64").toString("utf8").split(/\r?\n/);
}

function publicKey(base64) {
  const raw = Buffer.from(lines(base64)[1] ?? "", "base64");
  if (raw.length !== 42 || raw.toString("latin1", 0, 2) !== "Ed") throw new Error("not a minisign public key");
  const key = createPublicKey({ key: Buffer.concat([ED25519_SPKI_PREFIX, raw.subarray(10)]), format: "der", type: "spki" });
  return { keyId: raw.subarray(2, 10), key };
}

/**
 * Throws, saying why, unless `signature` (a Tauri `.sig` file's content) is a valid signature of
 * `data` by `publicKeyBase64` (the `pubkey` in tauri.conf.json).
 */
export function verify(data, signature, publicKeyBase64) {
  const { keyId, key } = publicKey(publicKeyBase64);
  const [, sigLine = "", trustedLine = "", globalLine = ""] = lines(signature);
  const sig = Buffer.from(sigLine, "base64");
  if (sig.length !== 74) throw new Error("malformed signature");
  if (!sig.subarray(2, 10).equals(keyId)) throw new Error("signed with a different key");
  const algorithm = sig.toString("latin1", 0, 2);
  // "ED": the file's BLAKE2b-512 hash is signed (what Tauri does); "Ed": the file itself.
  const message = algorithm === "ED" ? createHash("blake2b512").update(data).digest() : algorithm === "Ed" ? data : null;
  if (!message) throw new Error(`unknown signature algorithm ${algorithm}`);
  const fileSignature = sig.subarray(10);
  if (!verifyEd25519(null, message, key, fileSignature)) throw new Error("signature does not match the file");
  if (!trustedLine.startsWith(TRUSTED)) throw new Error("missing trusted comment");
  const comment = Buffer.from(trustedLine.slice(TRUSTED.length), "utf8");
  if (!verifyEd25519(null, Buffer.concat([fileSignature, comment]), key, Buffer.from(globalLine, "base64"))) {
    throw new Error("trusted comment signature does not match");
  }
}
