// F23: a tela Pessoas (artboard `People`). Sem React: testável no Node.

import { SEARCH_HASH } from "../search/query.ts";

/** O que o comando `people_list` devolve. */
export interface PersonCard {
  id: number;
  name: string;
  count: number;
  /** A foto mais recente com a pessoa; `null` se nenhuma. */
  cover: number | null;
}

const count = new Intl.NumberFormat("pt-BR");

/** "Ana Lúcia" e "ana lucia" se acham. */
function fold(text: string): string {
  return text.normalize("NFD").replace(/\p{M}/gu, "").toLowerCase().trim();
}

/** "Filtrar por nome": cada palavra digitada aparece no nome, em qualquer ordem. */
export function filterPeople(people: PersonCard[], text: string): PersonCard[] {
  const words = fold(text).split(/\s+/).filter((word) => word !== "");
  if (words.length === 0) {
    return people;
  }
  return people.filter((person) => {
    const name = fold(person.name);
    return words.every((word) => name.includes(word));
  });
}

/** "28 pessoas marcadas no Google Fotos, lidas dos JSONs do Takeout" */
export function peopleSubtitle(total: number): string {
  if (total === 0) {
    return "Ninguém marcado no Google Fotos";
  }
  if (total === 1) {
    return "1 pessoa marcada no Google Fotos, lida dos JSONs do Takeout";
  }
  return `${count.format(total)} pessoas marcadas no Google Fotos, lidas dos JSONs do Takeout`;
}

/** "812 itens" */
export function itemsLabel(total: number): string {
  return total === 1 ? "1 item" : `${count.format(total)} itens`;
}

/** A busca já filtrada pela pessoa: `#/buscar?pessoa=7`. */
export function personHash(id: number): string {
  return `${SEARCH_HASH}?pessoa=${id}`;
}

export function personFromHash(hash: string): number | null {
  const match = /^#\/buscar\?pessoa=(\d+)$/.exec(hash);
  return match === null ? null : Number(match[1]);
}

/** "AL" para "Ana Lúcia": o círculo de quem não tem capa. */
export function initials(name: string): string {
  const parts = name.trim().split(/\s+/).filter((part) => part !== "");
  const letters = parts.length > 1 ? [parts[0], parts[parts.length - 1]] : parts;
  return letters.map((part) => part.slice(0, 1).toUpperCase()).join("");
}

export const PEOPLE_HASH = "#/pessoas";
