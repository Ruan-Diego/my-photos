// F24: a tela Locais (artboard `Locations`). Sem React: testável no Node.

import { MONTHS } from "../timeline/timeline.ts";
import { SEARCH_HASH } from "../search/query.ts";
import { currentLocale, formatCount, t } from "../i18n/i18n.ts";

/** Um lugar com nome, do comando `locations_overview`. */
export interface Place {
  /** "São Paulo, SP, Brasil" */
  label: string;
  count: number;
  latitude: number;
  longitude: number;
  first: string | null;
  last: string | null;
  cover: number | null;
}

export interface Locations {
  located: number;
  unlocated: number;
  places: Place[];
}


/** "3.402 itens com coordenadas · 845 sem localização" */
export function locationsSubtitle(located: number, unlocated: number): string {
  return t("locations.subtitle", { items: t("count.items", { n: located }), unlocated: formatCount(unlocated) });
}

/** "São Paulo" de "São Paulo, SP, Brasil". */
export function placeName(label: string): string {
  return label.split(", ")[0];
}

/** "ago 2022" num mês só, "2022" num ano só, senão "2015–2024". */
export function period(first: string | null, last: string | null): string | null {
  if (first === null || last === null) {
    return null;
  }
  const [fromYear, toYear] = [first.slice(0, 4), last.slice(0, 4)];
  if (fromYear !== toYear) {
    return `${fromYear}–${toYear}`;
  }
  if (first.slice(5, 7) === last.slice(5, 7)) {
    return t("locations.monthYear", { month: MONTHS[Number(first.slice(5, 7)) - 1].slice(0, 3).toLowerCase(), year: fromYear });
  }
  return fromYear;
}

/** "SP, Brasil · 2015–2024" */
export function placeMeta(place: Place): string {
  const region = place.label.split(", ").slice(1).join(", ");
  const when = period(place.first, place.last);
  return [region, when].filter((part) => part !== null && part !== "").join(" · ");
}

/** O número no alfinete: "640", "1,8 mil", "12 mil". */
export function pinLabel(total: number): string {
  if (total < 1000) {
    return String(total);
  }
  const thousands = total / 1000;
  const shown = thousands < 10 ? Math.floor(thousands * 10) / 10 : Math.floor(thousands);
  return t("locations.thousands", { value: shown.toLocaleString(currentLocale()) });
}

/** 36 px para 1 item até 64 px para 2 mil ou mais, em escala logarítmica. */
export function pinSize(total: number): number {
  const t = Math.min(1, Math.log10(Math.max(1, total)) / Math.log10(2000));
  return Math.round(36 + t * 28);
}

/** Equiretangular, como o contorno em `world.ts`: x de 0 a 360, y de 0 a 180. */
export function project(latitude: number, longitude: number): { x: number; y: number } {
  return { x: longitude + 180, y: 90 - latitude };
}

export interface Pin {
  x: number;
  y: number;
  count: number;
  /** Os lugares do alfinete, o maior primeiro. */
  places: Place[];
}

/**
 * Lugares próximos na tela viram um alfinete só: o maior fica e engole quem estiver a
 * menos de `radius` px dele. `scale` é px por grau.
 */
export function pins(places: Place[], scale: number, radius = 44): Pin[] {
  const result: Pin[] = [];
  const sorted = [...places].sort((a, b) => b.count - a.count);
  for (const place of sorted) {
    const { x, y } = project(place.latitude, place.longitude);
    const near = result.find((pin) => Math.hypot((pin.x - x) * scale, (pin.y - y) * scale) < radius);
    if (near === undefined) {
      result.push({ x, y, count: place.count, places: [place] });
    } else {
      near.count += place.count;
      near.places.push(place);
    }
  }
  return result;
}

export interface View {
  /** Centro, em graus projetados. */
  x: number;
  y: number;
  /** px por grau. */
  scale: number;
}

export const MIN_SCALE = 1;
export const MAX_SCALE = 400;

export function clampScale(scale: number): number {
  return Math.min(MAX_SCALE, Math.max(MIN_SCALE, scale));
}

/** A vista que mostra todos os lugares numa área de `width` × `height` px, com margem. */
export function fitView(places: Place[], width: number, height: number): View {
  if (places.length === 0 || width <= 0 || height <= 0) {
    return { x: 180, y: 90, scale: clampScale(Math.min(width / 360, height / 180)) };
  }
  const points = places.map((place) => project(place.latitude, place.longitude));
  const xs = points.map((point) => point.x);
  const ys = points.map((point) => point.y);
  const [minX, maxX, minY, maxY] = [Math.min(...xs), Math.max(...xs), Math.min(...ys), Math.max(...ys)];
  // Pelo menos ~6° de lado: um lugar só não vira um zoom de rua.
  const spanX = Math.max(maxX - minX, 6);
  const spanY = Math.max(maxY - minY, 6);
  const scale = clampScale(Math.min((width * 0.8) / spanX, (height * 0.8) / spanY));
  return { x: (minX + maxX) / 2, y: (minY + maxY) / 2, scale };
}

/** A busca já filtrada pelo lugar: `#/buscar?local=S%C3%A3o%20Paulo%2C%20SP%2C%20Brasil`. */
export function placeHash(label: string): string {
  return `${SEARCH_HASH}?local=${encodeURIComponent(label)}`;
}

export function placeFromHash(hash: string): string | null {
  const match = /^#\/buscar\?local=(.+)$/.exec(hash);
  if (match === null) {
    return null;
  }
  try {
    return decodeURIComponent(match[1]);
  } catch {
    return null;
  }
}
