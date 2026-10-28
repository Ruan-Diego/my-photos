// F35: o cartão Backup das Configurações. Sem React: testável no Node.
import { dateText, t } from "../i18n/i18n.ts";
import { fileSize } from "../viewer/navigation.ts";

export interface BackupFile {
  name: string;
  bytes: number;
  kind: "database" | "export";
}

export interface BackupOverview {
  folder: string;
  files: BackupFile[];
  keep: number;
}

export interface ImportOutcome {
  favorites: number;
  unfavorites: number;
  notes: number;
  missing: number;
}

/** A hora no nome (`library-20261028140245.db`, UTC). */
export function stampDate(name: string): Date | null {
  const match = /-(\d{4})(\d{2})(\d{2})(\d{2})(\d{2})(\d{2})\./.exec(name);
  if (match === null) {
    return null;
  }
  const [year, month, day, hour, minute, second] = match.slice(1).map(Number);
  return new Date(Date.UTC(year, month - 1, day, hour, minute, second));
}

function when(name: string): string {
  const date = stampDate(name);
  return date === null ? name : dateText(date, { day: "numeric", month: "short", year: "numeric", hour: "2-digit", minute: "2-digit" });
}

/** "28 de out. de 2026, 14:02 · 7 cópias guardadas (1,2 GB)" ou "Nenhum ainda". */
export function lastBackupLine(overview: BackupOverview): string {
  const copies = overview.files.filter((file) => file.kind === "database");
  if (copies.length === 0) {
    return t("backup.none");
  }
  const total = copies.reduce((sum, file) => sum + file.bytes, 0);
  const kept = t("backup.copiesKept", { n: copies.length });
  return t("backup.lastLine", { when: when(copies[0].name), kept, size: fileSize(total), keep: String(overview.keep) });
}

/** As exportações, mais nova primeiro, com o rótulo da lista "Importar de". */
export function exportOptions(overview: BackupOverview): { name: string; label: string }[] {
  return overview.files.filter((file) => file.kind === "export").map((file) => ({ name: file.name, label: t("backup.exportOption", { when: when(file.name), name: file.name }) }));
}

/** O que a importação fez, em uma frase. */
export function importText(outcome: ImportOutcome): string {
  const parts: string[] = [];
  if (outcome.favorites > 0) {
    parts.push(t("backup.favoritesMarked", { n: outcome.favorites }));
  }
  if (outcome.unfavorites > 0) {
    parts.push(t("backup.favoritesUnmarked", { n: outcome.unfavorites }));
  }
  if (outcome.notes > 0) {
    parts.push(t("backup.notes", { n: outcome.notes }));
  }
  const done = parts.length === 0 ? t("backup.nothingChanged") : t("backup.imported", { parts: parts.join(", ") });
  if (outcome.missing === 0) {
    return t("backup.done", { done });
  }
  return t("backup.doneMissing", { done, missing: t("count.items", { n: outcome.missing }) });
}
