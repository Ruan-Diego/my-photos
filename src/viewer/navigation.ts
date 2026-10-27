// F19: o que o visualizador (artboard `Viewer`) diz e por onde ele anda. Sem React.

import { calendar, MONTHS, WEEKDAYS, type Item, type Month } from "../timeline/timeline.ts";

/** Uma mídia na ordem da grade, com o dia em que está. */
export interface Entry {
  item: Item;
  /** Posição dentro do dia, a partir de 1, e quantas o dia tem: "3 de 14". */
  position: number;
  dayCount: number;
}

/** A grade inteira em uma fila, na ordem em que aparece (filtro aplicado). */
export function order(months: Month[]): Entry[] {
  const entries: Entry[] = [];
  for (const month of months) {
    for (const day of month.days) {
      day.items.forEach((item, index) => entries.push({ item, position: index + 1, dayCount: day.items.length }));
    }
  }
  return entries;
}

export function indexOf(entries: Entry[], id: number): number {
  return entries.findIndex((entry) => entry.item.id === id);
}

/** "Qua, 10 de agosto de 2022 · 14:37"; "Sem data" quando não há. */
export function dateLine(item: Item): string {
  if (item.time === null) {
    return "Sem data";
  }
  const when = calendar(item.time, item.offset);
  const hour = `${String(when.hour).padStart(2, "0")}:${String(when.minute).padStart(2, "0")}`;
  return `${WEEKDAYS[when.weekday]}, ${when.day} de ${MONTHS[when.month].toLowerCase()} de ${when.year} · ${hour}`;
}

/** "3 de 14": a posição no dia, como no canvas. O local entra com a F24. */
export function positionLine(entry: Entry): string {
  return `${entry.position} de ${entry.dayCount}`;
}

/** Até `size` vizinhas para a tira de baixo, com a atual o mais ao centro possível. */
export function stripWindow(index: number, total: number, size = 12): [number, number] {
  if (total <= size) {
    return [0, total];
  }
  const start = Math.min(Math.max(0, index - Math.floor(size / 2) + 1), total - size);
  return [start, start + size];
}

/** Onde o visualizador foi aberto: a grade (`fotos`), a busca (`buscar`, F22) ou os favoritos (F26). */
export type ViewerBase = "fotos" | "buscar" | "favoritos";

/** `#/fotos/1066` → 1066; `#/buscar/1066` com `base` `buscar`. */
export function openedId(hash: string, base: ViewerBase = "fotos"): number | null {
  const match = new RegExp(`^#/${base}/(\\d+)$`).exec(hash);
  return match === null ? null : Number(match[1]);
}

export function viewerHash(id: number, base: ViewerBase = "fotos"): string {
  return `#/${base}/${id}`;
}

export const TIMELINE_HASH = "#/fotos";

/** Para onde voltar: a grade ou a busca, com o que estava nela. */
export function backHash(base: ViewerBase): string {
  return `#/${base}`;
}

export function backLabel(base: ViewerBase): string {
  return base === "buscar" ? "Voltar para a busca" : base === "favoritos" ? "Voltar para Favoritos" : "Voltar para Fotos";
}

export type Zoom = "fit" | "actual";

/** O que fica no lugar do original quando o app não o mostra. */
export function fallbackNote(info: { video: boolean; extension: string } | null): string | null {
  if (info === null) {
    return null;
  }
  if (info.video) {
    return "O player de vídeo chega com a F20. Por enquanto, só a capa.";
  }
  return `Arquivos .${info.extension.toUpperCase()} ainda não abrem no app. Esta é a miniatura.`;
}

const decimal = new Intl.NumberFormat("pt-BR", { maximumFractionDigits: 1 });

/** "3,4 MB", "84 MB", "1,2 GB". */
export function fileSize(bytes: number): string {
  const units = ["B", "KB", "MB", "GB", "TB"];
  let value = bytes;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit += 1;
  }
  const rounded = value >= 10 || unit === 0 ? Math.round(value) : Math.round(value * 10) / 10;
  return `${decimal.format(rounded)} ${units[unit]}`;
}

/** "VID_20220810_170512.mp4 · 1920 × 1080 · 84 MB": o que se sabe, nessa ordem. */
export function videoLine(info: { filename: string; width: number | null; height: number | null; sizeBytes: number | null }): string {
  const parts = [info.filename];
  if (info.width !== null && info.height !== null) {
    parts.push(`${info.width} × ${info.height}`);
  }
  if (info.sizeBytes !== null) {
    parts.push(fileSize(info.sizeBytes));
  }
  return parts.join(" · ");
}

/** Segundos do player → "0:42", "1:02:03". */
export function clock(seconds: number): string {
  const total = Number.isFinite(seconds) && seconds > 0 ? Math.floor(seconds) : 0;
  const hours = Math.floor(total / 3600);
  const minutes = Math.floor((total % 3600) / 60);
  const rest = String(total % 60).padStart(2, "0");
  return hours > 0 ? `${hours}:${String(minutes).padStart(2, "0")}:${rest}` : `${minutes}:${rest}`;
}

/** Passo das setas no player (canvas: "← →: voltar/avançar 5 s"). */
export const SEEK_STEP = 5;

export function seek(current: number, delta: number, duration: number): number {
  const end = Number.isFinite(duration) && duration > 0 ? duration : Math.max(0, current + delta);
  return Math.min(end, Math.max(0, current + delta));
}

/** Por que o vídeo não toca; `null` enquanto pode tocar. */
export function videoProblem(info: { playable: boolean; extension: string } | null, failed: boolean): string | null {
  if (info === null) {
    return null;
  }
  if (!info.playable) {
    return `Vídeos .${info.extension.toUpperCase()} ainda não tocam no app. Esta é a capa.`;
  }
  if (failed) {
    return "O Windows não tem o codec deste vídeo (às vezes HEVC/H.265). Esta é a capa.";
  }
  return null;
}
