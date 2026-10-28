// F35: o cartão Backup das Configurações. Sem React: testável no Node.
import { dateText, formatCount, t } from "../i18n/i18n.ts";
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
    return "Nenhum ainda. O app faz um por dia ao abrir.";
  }
  const total = copies.reduce((sum, file) => sum + file.bytes, 0);
  const kept = copies.length === 1 ? "1 cópia guardada" : `${copies.length} cópias guardadas`;
  return `${when(copies[0].name)} · ${kept} (${fileSize(total)}), as ${overview.keep} mais novas ficam`;
}

/** As exportações, mais nova primeiro, com o rótulo da lista "Importar de". */
export function exportOptions(overview: BackupOverview): { name: string; label: string }[] {
  return overview.files.filter((file) => file.kind === "export").map((file) => ({ name: file.name, label: `${when(file.name)} · ${file.name}` }));
}

/** O que a importação fez, em uma frase. */
export function importText(outcome: ImportOutcome): string {
  const parts: string[] = [];
  if (outcome.favorites > 0) {
    parts.push(outcome.favorites === 1 ? "1 favorito marcado" : `${formatCount(outcome.favorites)} favoritos marcados`);
  }
  if (outcome.unfavorites > 0) {
    parts.push(outcome.unfavorites === 1 ? "1 favorito desmarcado" : `${formatCount(outcome.unfavorites)} favoritos desmarcados`);
  }
  if (outcome.notes > 0) {
    parts.push(outcome.notes === 1 ? "1 nota" : `${formatCount(outcome.notes)} notas`);
  }
  const done = parts.length === 0 ? "Nada mudou: tudo já estava igual" : `Importado: ${parts.join(", ")}`;
  if (outcome.missing === 0) {
    return `${done}.`;
  }
  return `${done}. ${t("count.items", { n: outcome.missing })} do arquivo não estão nesta biblioteca.`;
}
