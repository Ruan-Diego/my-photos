// F33: contraste WCAG 2.1 entre dois tokens do tema. Sem React: testável no Node.

/** Luminância relativa de `#rrggbb`. */
export function luminance(hex: string): number {
  const value = hex.replace("#", "");
  const [r, g, b] = [0, 2, 4].map((at) => {
    const channel = parseInt(value.slice(at, at + 2), 16) / 255;
    return channel <= 0.03928 ? channel / 12.92 : ((channel + 0.055) / 1.055) ** 2.4;
  });
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}

export function contrast(a: string, b: string): number {
  const [light, dark] = [luminance(a), luminance(b)].sort((x, y) => y - x);
  return (light + 0.05) / (dark + 0.05);
}

/** Os tokens `--nome: #hex` de cada bloco do `tokens.css`: claro (`:root`) e escuro. */
export function themes(css: string): { light: Map<string, string>; dark: Map<string, string> } {
  const block = (start: number) => {
    const body = css.slice(css.indexOf("{", start) + 1, css.indexOf("}", start));
    return new Map([...body.matchAll(/--([\w-]+):\s*(#[0-9a-fA-F]{6})\b/g)].map((match) => [match[1], match[2]]));
  };
  const light = block(css.indexOf(":root {"));
  // O escuro herda o que não redefine (o visualizador é igual nos dois temas).
  const dark = new Map([...light, ...block(css.indexOf(':root[data-theme="dark"]'))]);
  return { light, dark };
}

/**
 * Pares texto/fundo que as telas usam, com o mínimo AA: 4,5 para texto, 3 para contorno de
 * foco e ícones. Os fundos de foto (selos em rgba) ficam fora.
 */
export const PAIRS: readonly [string, string, number][] = [
  ["ink", "ground", 4.5],
  ["ink", "surface", 4.5],
  ["ink", "side", 4.5],
  ["ink", "sunken", 4.5],
  ["ink-2", "surface", 4.5],
  ["ink-2", "side", 4.5],
  ["muted", "ground", 4.5],
  ["muted", "surface", 4.5],
  ["muted", "side", 4.5],
  ["muted", "side-card", 4.5],
  ["muted", "sunken", 4.5],
  ["accent", "ground", 4.5],
  ["accent", "surface", 4.5],
  ["ground", "ink", 4.5],
  ["on-accent", "accent-fill", 4.5],
  ["warn-fg", "warn-bg", 4.5],
  ["ok-fg", "ok-bg", 4.5],
  ["ink", "neutral-bg", 4.5],
  ["console-ink", "console", 4.5],
  ["console-warn", "console", 4.5],
  ["viewer-ink", "viewer-ground", 4.5],
  ["viewer-soft", "viewer-panel", 4.5],
  ["viewer-muted", "viewer-ground", 4.5],
  ["viewer-muted", "viewer-panel", 4.5],
  ["viewer-ink", "viewer-button", 4.5],
  ["viewer-ok-fg", "viewer-ok-bg", 4.5],
  ["viewer-warn-fg", "viewer-warn-bg", 4.5],
  ["viewer-accent", "viewer-ground", 4.5],
  ["accent-fill", "ground", 3],
  ["accent-fill", "surface", 3],
];
