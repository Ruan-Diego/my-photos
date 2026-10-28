// F29: a tela Duplicatas (artboard `Duplicates`). Só informativo. Sem React.

import { FAVORITE, VIDEO, type Item, type TimelineData } from "../timeline/timeline.ts";
import { fileSize } from "../viewer/navigation.ts";
import { formatCount, list, t } from "../i18n/i18n.ts";

export type Kind = "identical" | "similar";

export interface Copy {
  id: number;
  path: string;
  sizeBytes: number | null;
  width: number | null;
  height: number | null;
  takenAt: string | null;
}

export interface Group {
  kind: Kind;
  copies: Copy[];
}

export interface Duplicates {
  groups: Group[];
  hashed: number;
  photos: number;
}

export type KindFilter = "all" | Kind;

const MONTHS = list("common.shortMonths");

/** "Todas · 214", "Idênticas · 187", "Parecidas · 27". */
export function kindFilters(groups: Group[]): { id: KindFilter; label: string }[] {
  const of = (kind: Kind) => groups.filter((group) => group.kind === kind).length;
  return [
    { id: "all", label: t("duplicates.all", { count: formatCount(groups.length) }) },
    { id: "identical", label: t("duplicates.identical", { count: formatCount(of("identical")) }) },
    { id: "similar", label: t("duplicates.similar", { count: formatCount(of("similar")) }) },
  ];
}

export function filterGroups(groups: Group[], filter: KindFilter): Group[] {
  return filter === "all" ? groups : groups.filter((group) => group.kind === filter);
}

export const KIND_LABEL: Record<Kind, string> = { identical: t("duplicates.identicalKind"), similar: t("duplicates.similarKind") };

export function fileName(path: string): string {
  return path.split("/").pop() ?? path;
}

/** `takeout-…/Takeout/Google Fotos/Viagem/a.jpg` → `Google Fotos\Viagem`. */
export function folderLabel(path: string): string {
  const parts = path.split("/").filter((part) => part !== "");
  parts.pop();
  return parts.slice(-2).join("\\") || "\\";
}

/** "foto001.jpg · 2 cópias"; "IMG_20220811_101522 · 3 versões". */
export function groupTitle(group: Group): string {
  const first = fileName(group.copies[0].path);
  const n = group.copies.length;
  return group.kind === "identical" ? t("duplicates.copies", { name: first, count: n }) : t("duplicates.versions", { name: first.replace(/\.[^.]+$/, ""), count: n });
}

/** "3,1 MB repetidos"; para parecidas, se os tamanhos batem. */
export function groupSaving(group: Group): string {
  if (group.kind === "identical") {
    const size = group.copies[0].sizeBytes ?? 0;
    return t("duplicates.repeated", { size: fileSize(size * (group.copies.length - 1)) });
  }
  const sizes = new Set(group.copies.map((copy) => `${copy.width}×${copy.height}`));
  return sizes.size > 1 ? t("duplicates.differentSizes") : t("duplicates.sameSize");
}

/** "10 ago 2022" (UTC, como o índice guarda). */
export function shortDate(iso: string | null): string | null {
  if (iso === null) {
    return null;
  }
  const date = new Date(iso);
  return Number.isNaN(date.getTime()) ? null : t("common.shortDate", { day: date.getUTCDate(), month: MONTHS[date.getUTCMonth()], year: String(date.getUTCFullYear()) });
}

/** Idênticas: "3,1 MB · 10 ago 2022"; parecidas: "4032 × 3024 · 3,8 MB". */
export function copyMeta(kind: Kind, copy: Copy): string {
  const size = copy.sizeBytes === null ? null : fileSize(copy.sizeBytes);
  const parts =
    kind === "identical"
      ? [size, shortDate(copy.takenAt)]
      : [copy.width !== null && copy.height !== null ? `${copy.width} × ${copy.height}` : null, size];
  return parts.filter((part) => part !== null).join(" · ");
}

/** Os itens da grade para o visualizador, na ordem dos grupos (sem repetir). */
export function viewerItems(groups: Group[], data: TimelineData | null): Item[] {
  if (data === null) {
    return [];
  }
  const index = new Map(data.ids.map((id, at) => [id, at]));
  const seen = new Set<number>();
  const items: Item[] = [];
  for (const group of groups) {
    for (const copy of group.copies) {
      const at = index.get(copy.id);
      if (at === undefined || seen.has(copy.id)) {
        continue;
      }
      seen.add(copy.id);
      items.push({
        id: copy.id,
        ratio: data.ratios[at],
        video: (data.flags[at] & VIDEO) !== 0,
        favorite: (data.flags[at] & FAVORITE) !== 0,
        duration: data.durations[at],
        time: data.times[at],
        offset: data.offsets[at],
      });
    }
  }
  return items;
}

export function emptyDuplicates(filter: KindFilter, hashed: number, photos: number): { title: string; detail: string } {
  if (filter === "similar" && hashed < photos) {
    return {
      title: t("duplicates.noSimilarYet"),
      detail: t("duplicates.partlyCompared", { hashed: formatCount(hashed), photos: formatCount(photos), action: t("disk.scanNew") }),
    };
  }
  return { title: t("duplicates.noneTitle"), detail: t("duplicates.noneDetail") };
}
