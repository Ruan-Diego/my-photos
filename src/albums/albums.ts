// F25: a tela Álbuns (artboard `Albums`). Sem React: testável no Node.

import { period } from "../locations/locations.ts";
import { SEARCH_HASH } from "../search/query.ts";
import { collator, t } from "../i18n/i18n.ts";

/** O que o comando `albums_list` devolve. */
export interface AlbumCard {
  id: number;
  name: string;
  count: number;
  first: string | null;
  last: string | null;
  /** Até três fotos, a mais recente primeiro: a pilha da capa. */
  covers: number[];
}

export type AlbumOrder = "recent" | "name" | "size";

export const ORDERS: { value: AlbumOrder; label: string }[] = [
  { value: "recent", label: "Mais recentes" },
  { value: "name", label: "Nome" },
  { value: "size", label: "Mais itens" },
];

const names = collator({ sensitivity: "base", numeric: true });

export function sortAlbums(albums: AlbumCard[], order: AlbumOrder): AlbumCard[] {
  const sorted = [...albums];
  if (order === "name") {
    sorted.sort((a, b) => names.compare(a.name, b.name));
  } else if (order === "size") {
    sorted.sort((a, b) => b.count - a.count || names.compare(a.name, b.name));
  } else {
    // Sem data vai para o fim.
    sorted.sort((a, b) => (b.last ?? "").localeCompare(a.last ?? "") || names.compare(a.name, b.name));
  }
  return sorted;
}


/** "124 itens · ago 2022" */
export function albumMeta(album: AlbumCard): string {
  const items = t("count.items", { n: album.count });
  const when = period(album.first, album.last);
  return when === null ? items : `${items} · ${when}`;
}

/** As fotos do álbum, na busca: `#/buscar?album=5`. */
export function albumHash(id: number): string {
  return `${SEARCH_HASH}?album=${id}`;
}

export function albumFromHash(hash: string): number | null {
  const match = /^#\/buscar\?album=(\d+)$/.exec(hash);
  return match === null ? null : Number(match[1]);
}

export const ORDER_KEY = "myphotos.albums.order";

export function loadOrder(storage: Pick<Storage, "getItem"> | null): AlbumOrder {
  try {
    const saved = storage?.getItem(ORDER_KEY);
    return ORDERS.some((order) => order.value === saved) ? (saved as AlbumOrder) : "recent";
  } catch {
    return "recent";
  }
}
