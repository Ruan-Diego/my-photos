// Seções da barra lateral, na ordem do canvas (artboard `Sidebar`).

import { t } from "../i18n/i18n.ts";

export type SectionId =
  | "timeline"
  | "search"
  | "folders"
  | "albums"
  | "people"
  | "locations"
  | "favorites"
  | "duplicates"
  | "problems"
  | "trash"
  | "settings";

export interface Section {
  id: SectionId;
  label: string;
  /** Hash route: `#/fotos`. */
  slug: string;
  /** `hidden`: tela sem item na lateral (a busca abre pela caixa da timeline). */
  group: "main" | "care" | "hidden";
  /** Feature that builds the screen. */
  feature: string;
  /** SVG path, 24×24, traço 1.7. */
  icon: string;
}

export const SECTIONS: readonly Section[] = [
  { id: "timeline", label: t("section.timeline"), slug: "fotos", group: "main", feature: "F18", icon: "M4 4h7v7H4zM13 4h7v7h-7zM4 13h7v7H4zM13 13h7v7h-7z" },
  { id: "folders", label: t("section.folders"), slug: "pastas", group: "main", feature: "F48", icon: "M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" },
  { id: "albums", label: t("section.albums"), slug: "albuns", group: "main", feature: "F25", icon: "M7 3h12v14H7zM4 7v13h13M7 13l3-3 3 3 2-2 4 4" },
  { id: "people", label: t("section.people"), slug: "pessoas", group: "main", feature: "F23", icon: "M12 12a4 4 0 1 0 0-8 4 4 0 0 0 0 8zM4 21a8 8 0 0 1 16 0" },
  { id: "locations", label: t("section.locations"), slug: "locais", group: "main", feature: "F24", icon: "M12 21s7-6.5 7-12a7 7 0 0 0-14 0c0 5.5 7 12 7 12zM12 11.5a2.5 2.5 0 1 0 0-5 2.5 2.5 0 0 0 0 5z" },
  { id: "favorites", label: t("section.favorites"), slug: "favoritos", group: "main", feature: "F26", icon: "M12 20s-7-4.4-7-10a4 4 0 0 1 7-2.6A4 4 0 0 1 19 10c0 5.6-7 10-7 10z" },
  { id: "duplicates", label: t("section.duplicates"), slug: "duplicatas", group: "care", feature: "F29", icon: "M8 8h12v12H8zM4 16V4h12" },
  { id: "problems", label: t("section.problems"), slug: "problemas", group: "care", feature: "F28", icon: "M12 3l9 16H3zM12 10v4M12 17v.01" },
  { id: "trash", label: t("section.trash"), slug: "lixeira", group: "care", feature: "AD-009", icon: "M4 7h16M10 11v6M14 11v6M6 7l1 13h10l1-13M9 7V4h6v3" },
  { id: "search", label: t("section.search"), slug: "buscar", group: "hidden", feature: "F22", icon: "M11 18a7 7 0 1 0 0-14 7 7 0 0 0 0 14zM20 20l-4-4" },
  { id: "settings", label: t("section.settings"), slug: "configuracoes", group: "care", feature: "F31", icon: "M4 6h9M17 6h3M4 12h3M11 12h9M4 18h11M19 18h1M15 4v4M9 10v4M17 16v4" },
];

export const DEFAULT_SECTION: SectionId = "timeline";

export function sectionById(id: SectionId): Section {
  return SECTIONS.find((section) => section.id === id) ?? SECTIONS[0];
}

/** `#/pessoas` → people; empty or unknown → Fotos. */
export function sectionFromHash(hash: string): SectionId {
  const slug = hash.replace(/^#\/?/, "").split(/[/?]/)[0].toLowerCase();
  return SECTIONS.find((section) => section.slug === slug)?.id ?? DEFAULT_SECTION;
}

export function hashFor(id: SectionId): string {
  return `#/${sectionById(id).slug}`;
}
