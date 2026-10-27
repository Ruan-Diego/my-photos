// F22: a busca (artboard `Search`): o que a tela manda para o Rust e o que ela diz.
// Sem React: testável no Node.

import { MONTHS } from "../timeline/timeline.ts";

/** O que o comando `search_media` recebe (`SearchQuery` do F14). */
export interface SearchQuery {
  text: string;
  mediaType: "photo" | "video" | null;
  /** ISO, inclusive. */
  from: string | null;
  /** ISO, exclusive. */
  until: string | null;
  people: number[];
  folder: string | null;
  /** F24: nome do lugar, ou parte dele. */
  place: string | null;
  /** F25: só deste álbum. */
  album: number | null;
  favoritesOnly: boolean;
}

/** O que o comando `search_media` devolve. */
export interface SearchIds {
  ids: number[];
  photos: number;
  videos: number;
}

export interface PersonOption {
  id: number;
  name: string;
  count: number;
}

export interface AlbumOption {
  id: number;
  name: string;
}

export interface SearchOptions {
  people: PersonOption[];
  folders: string[];
  albums: AlbumOption[];
}

/** O que a coluna de filtros guarda, como o usuário digitou. */
export interface Filters {
  /** "01/08/2022" */
  from: string;
  until: string;
  photos: boolean;
  videos: boolean;
  favoritesOnly: boolean;
  people: number[];
  folder: string | null;
  /** Campo Local, como digitado, ou o lugar clicado em Locais (F24). */
  place: string;
  /** O álbum clicado em Álbuns (F25); não tem campo, só o chip. */
  album: number | null;
}

export const NO_FILTERS: Filters = { from: "", until: "", photos: true, videos: true, favoritesOnly: false, people: [], folder: null, place: "", album: null };

/** "01/08/2022" ou "1/8/2022" → "2022-08-01"; `null` se não for uma data. */
export function parseDate(text: string): string | null {
  const match = /^\s*(\d{1,2})\/(\d{1,2})\/(\d{4})\s*$/.exec(text);
  if (match === null) {
    return null;
  }
  const [day, month, year] = [Number(match[1]), Number(match[2]), Number(match[3])];
  const date = new Date(Date.UTC(year, month - 1, day));
  if (date.getUTCFullYear() !== year || date.getUTCMonth() !== month - 1 || date.getUTCDate() !== day) {
    return null;
  }
  return date.toISOString().slice(0, 10);
}

/** "2022-08-31" → "2022-09-01": o "Até" inclui o dia inteiro. */
export function dayAfter(iso: string): string {
  const date = new Date(`${iso}T00:00:00Z`);
  date.setUTCDate(date.getUTCDate() + 1);
  return date.toISOString().slice(0, 10);
}

/** Campo de data preenchido com algo que não é data: fica marcado e não filtra. */
export function invalidDate(text: string): boolean {
  return text.trim() !== "" && parseDate(text) === null;
}

/** Nenhum tipo marcado: não há o que mostrar, nem vale perguntar ao Rust. */
export function noType(filters: Filters): boolean {
  return !filters.photos && !filters.videos;
}

export function toQuery(text: string, filters: Filters): SearchQuery {
  const from = parseDate(filters.from);
  const until = parseDate(filters.until);
  return {
    text: text.trim(),
    mediaType: filters.photos === filters.videos ? null : filters.photos ? "photo" : "video",
    from,
    until: until === null ? null : dayAfter(until),
    people: filters.people,
    folder: filters.folder,
    place: filters.place.trim() === "" ? null : filters.place.trim(),
    album: filters.album,
    favoritesOnly: filters.favoritesOnly,
  };
}

/** Sem texto e sem filtro: a tela convida a buscar em vez de listar tudo. */
export function isBlank(text: string, filters: Filters): boolean {
  const query = toQuery(text, filters);
  return (
    query.text === "" &&
    query.mediaType === null &&
    !noType(filters) &&
    query.from === null &&
    query.until === null &&
    query.people.length === 0 &&
    query.folder === null &&
    query.place === null &&
    query.album === null &&
    !query.favoritesOnly
  );
}

/** "ago 2022" */
function shortMonth(iso: string): string {
  return `${MONTHS[Number(iso.slice(5, 7)) - 1].slice(0, 3).toLowerCase()} ${iso.slice(0, 4)}`;
}

function lastDayOfMonth(iso: string): boolean {
  return dayAfter(iso).slice(8, 10) === "01";
}

/** O chip da data: "ago 2022" quando é um mês inteiro, senão as datas. */
export function dateChip(filters: Filters): string | null {
  const from = parseDate(filters.from);
  const until = parseDate(filters.until);
  if (from !== null && until !== null) {
    if (from.slice(0, 7) === until.slice(0, 7) && from.endsWith("-01") && lastDayOfMonth(until)) {
      return shortMonth(from);
    }
    return `${filters.from.trim()} a ${filters.until.trim()}`;
  }
  if (from !== null) {
    return `desde ${filters.from.trim()}`;
  }
  if (until !== null) {
    return `até ${filters.until.trim()}`;
  }
  return null;
}

export type ChipKey = "date" | "type" | "favorites" | "folder" | "place" | "album" | `person:${number}`;

/** Um chip por filtro ativo, na ordem da coluna; o X tira só aquele filtro. */
export function chips(filters: Filters, people: PersonOption[], albums: AlbumOption[] = []): { key: ChipKey; label: string }[] {
  const result: { key: ChipKey; label: string }[] = [];
  if (filters.album !== null) {
    result.push({ key: "album", label: albums.find((album) => album.id === filters.album)?.name ?? "Álbum" });
  }
  const date = dateChip(filters);
  if (date !== null) {
    result.push({ key: "date", label: date });
  }
  if (filters.photos !== filters.videos) {
    result.push({ key: "type", label: filters.photos ? "Só fotos" : "Só vídeos" });
  }
  if (filters.favoritesOnly) {
    result.push({ key: "favorites", label: "Favoritos" });
  }
  for (const id of filters.people) {
    const person = people.find((option) => option.id === id);
    result.push({ key: `person:${id}`, label: person?.name ?? "Pessoa" });
  }
  if (filters.place.trim() !== "") {
    // "São Paulo, SP, Brasil" vira "São Paulo".
    result.push({ key: "place", label: filters.place.trim().split(", ")[0] });
  }
  if (filters.folder !== null) {
    result.push({ key: "folder", label: folderLabel(filters.folder) });
  }
  return result;
}

export function removeChip(filters: Filters, key: ChipKey): Filters {
  switch (key) {
    case "date":
      return { ...filters, from: "", until: "" };
    case "type":
      return { ...filters, photos: true, videos: true };
    case "favorites":
      return { ...filters, favoritesOnly: false };
    case "folder":
      return { ...filters, folder: null };
    case "place":
      return { ...filters, place: "" };
    case "album":
      return { ...filters, album: null };
    default: {
      const id = Number(key.slice("person:".length));
      return { ...filters, people: filters.people.filter((person) => person !== id) };
    }
  }
}

/** "Takeout/Google Fotos/Viagem" → "Google Fotos\Viagem", como no canvas. */
export function folderLabel(path: string): string {
  return path.replace(/^Takeout\//i, "").replace(/\//g, "\\");
}

const count = new Intl.NumberFormat("pt-BR");

/** "48 resultados" */
export function resultsTitle(total: number): string {
  if (total === 0) {
    return "Nenhum resultado";
  }
  return total === 1 ? "1 resultado" : `${count.format(total)} resultados`;
}

/** `#/buscar`: a tela (F22); `#/buscar/1066` abre o visualizador por cima. */
export const SEARCH_HASH = "#/buscar";
