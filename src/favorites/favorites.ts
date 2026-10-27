// F26: a tela Favoritos (artboard `Favorites`) e a marcação no visualizador. Sem React.

import { FAVORITE, group, UNDATED, type Filter, type Item, type TimelineData } from "../timeline/timeline.ts";

export type FavoritesFilter = Extract<Filter, "all" | "photos" | "videos">;

export const FAVORITES_FILTERS: readonly { id: FavoritesFilter; label: string }[] = [
  { id: "all", label: "Tudo" },
  { id: "photos", label: "Fotos" },
  { id: "videos", label: "Vídeos" },
];

const count = new Intl.NumberFormat("pt-BR");

/** "96 itens · marcados só no app, o arquivo original não muda" */
export function favoritesSubtitle(total: number): string {
  const items = total === 1 ? "1 item" : `${count.format(total)} itens`;
  return `${items} · marcados só no app, o arquivo original não muda`;
}

export function favoriteCount(data: TimelineData): number {
  return data.flags.reduce((total, flags) => total + ((flags & FAVORITE) !== 0 ? 1 : 0), 0);
}

export interface YearGroup {
  key: string;
  /** "2022", ou "Sem data". */
  title: string;
  items: Item[];
}

/**
 * Favoritos por ano, mais novo primeiro. `keep` segura na tela os itens que
 * acabaram de ser desmarcados no visualizador, até ele fechar.
 */
export function favoritesByYear(data: TimelineData, filter: FavoritesFilter, keep: ReadonlySet<number> = new Set()): YearGroup[] {
  const years: YearGroup[] = [];
  for (const month of group(data, keep.size === 0 ? "favorites" : "all")) {
    const key = month.year === null ? "undated" : String(month.year);
    let year = years[years.length - 1];
    if (year === undefined || year.key !== key) {
      year = { key, title: month.year === null ? UNDATED : String(month.year), items: [] };
      years.push(year);
    }
    for (const day of month.days) {
      for (const item of day.items) {
        const listed = item.favorite || keep.has(item.id);
        if (listed && (filter === "all" || (filter === "videos") === item.video)) {
          year.items.push(item);
        }
      }
    }
  }
  return years.filter((year) => year.items.length > 0);
}

/** Os mesmos dados com um item marcado ou desmarcado (o resto é compartilhado). */
export function withFavorite(data: TimelineData, id: number, favorite: boolean): TimelineData {
  const index = data.ids.indexOf(id);
  if (index < 0 || ((data.flags[index] & FAVORITE) !== 0) === favorite) {
    return data;
  }
  const flags = data.flags.slice();
  flags[index] = favorite ? flags[index] | FAVORITE : flags[index] & ~FAVORITE;
  return { ...data, flags };
}

export function emptyFavorites(filter: FavoritesFilter, total: number): { title: string; detail: string } {
  if (total > 0) {
    return filter === "videos"
      ? { title: "Nenhum vídeo favorito", detail: "Os vídeos marcados com o coração aparecem aqui." }
      : { title: "Nenhuma foto favorita", detail: "As fotos marcadas com o coração aparecem aqui." };
  }
  return {
    title: "Nenhum favorito ainda",
    detail: "Toque no coração ao ver uma foto ou vídeo. As favoritas do Google Fotos também aparecem aqui.",
  };
}
