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
