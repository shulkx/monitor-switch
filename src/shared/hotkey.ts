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
