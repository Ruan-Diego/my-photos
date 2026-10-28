// F26: a tela Favoritos (artboard `Favorites`) e a marcação no visualizador. Sem React.

import { t } from "../i18n/i18n.ts";
import { FAVORITE, group, UNDATED, type Filter, type Item, type TimelineData } from "../timeline/timeline.ts";

export type FavoritesFilter = Extract<Filter, "all" | "photos" | "videos">;

export const FAVORITES_FILTERS: readonly { id: FavoritesFilter; label: string }[] = [
  { id: "all", label: t("timeline.filterAll") },
  { id: "photos", label: t("timeline.filterPhotos") },
  { id: "videos", label: t("timeline.filterVideos") },
];

/** "96 itens · marcados só no app, o arquivo original não muda" */
export function favoritesSubtitle(total: number): string {
  return t("favorites.subtitle", { items: t("count.items", { n: total }) });
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
      ? { title: t("favorites.noVideosTitle"), detail: t("favorites.noVideosDetail") }
      : { title: t("favorites.noPhotosTitle"), detail: t("favorites.noPhotosDetail") };
  }
  return { title: t("favorites.noneTitle"), detail: t("timeline.empty.favoritesDetail") };
}
