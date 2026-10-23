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

/** `#/fotos/1066` → 1066. */
export function openedId(hash: string): number | null {
  const match = /^#\/fotos\/(\d+)$/.exec(hash);
  return match === null ? null : Number(match[1]);
}

export function viewerHash(id: number): string {
  return `#/fotos/${id}`;
}

export const TIMELINE_HASH = "#/fotos";

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
