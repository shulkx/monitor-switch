// Samples made with throwaway keys: `tauri signer generate --ci`, then `tauri signer sign` on DATA.
import assert from "node:assert/strict";
import { test } from "node:test";

import { verify } from "./minisign.mjs";

const DATA = Buffer.from("MonitorSwitch test payload\n");
const KEY =
  "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IEJDMUMwNEYyNTQ5Q0IzNjkKUldScHM1eFU4Z1FjdkZrVmFzRHk0c2JCVVpVb09aZHhtUUpTNDBLV1JZa1lhTTFUaWF1WEl1amIK";
const OTHER_KEY =
  "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IDlGQkFBRjlBOTkyNTdFNzMKUldSemZpV1ptcSs2bjQ4S2EvaWMya2NabjhQUGNVK0g1TkJmbTU5S2RPaTNXNkN2WlJOMmxTQm8K";
const SIG =
  "dW50cnVzdGVkIGNvbW1lbnQ6IHNpZ25hdHVyZSBmcm9tIHRhdXJpIHNlY3JldCBrZXkKUlVScHM1eFU4Z1FjdklLOXZXbytlUUtWVENaa2xLRU1VWFlISTFabzlJb3hJUXNVU1libHVzWDhoa2ZpOXFkUng1NlBxdEw3WnFISi9QRWYrdGl3VWJGQkxZQUNNZW1kc0FFPQp0cnVzdGVkIGNvbW1lbnQ6IHRpbWVzdGFtcDoxNzk0NDc4NTcyCWZpbGU6aGVsbG8udHh0CklpSW83M3EzZllBaWdwM3k4MjBndUlTUmpmV3BDb3drSXh4QzdOMjVabzJveXBUeXI1Vi8wb2dTWjZ6anRwRXFCdlZ6a0xzalRJNktCZnhWRmpMN0R3PT0K";

/** The signature with its trusted comment changed, which only the global signature catches. */
function withComment(comment) {
  const text = Buffer.from(SIG, "base64").toString("utf8").split("\n");
  text[2] = `trusted comment: ${comment}`;
  return Buffer.from(text.join("\n")).toString("base64");
}

test("accepts the file it signed", () => {
  assert.doesNotThrow(() => verify(DATA, SIG, KEY));
});

test("rejects a changed file", () => {
  assert.throws(() => verify(Buffer.from("MonitorSwitch test payload!\n"), SIG, KEY), /does not match the file/);
});

test("rejects another key", () => {
  assert.throws(() => verify(DATA, SIG, OTHER_KEY), /different key/);
});

test("rejects a changed trusted comment", () => {
  assert.throws(() => verify(DATA, withComment("timestamp:1\tfile:evil.exe"), KEY), /trusted comment/);
});

test("rejects something that isn't a signature", () => {
  assert.throws(() => verify(DATA, Buffer.from("hello").toString("base64"), KEY), /malformed/);
});
