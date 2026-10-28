// F28: a tela Problemas (artboard `Problems`): itens não reconhecidos, relatório da
// indexação e diagnóstico. Sem React.

import { fileSize } from "../viewer/navigation.ts";
import { formatCount } from "../i18n/i18n.ts";

export type Status = "pending" | "resolved" | "ignored";

export interface Problem {
  path: string;
  reason: string;
  action: string;
  status: Status;
}

export interface Report {
  media: number;
  dateFromJson: number;
  dateFromExif: number;
  dateFromFile: number;
  jsonInOtherPart: number;
  lastDurationSeconds: number | null;
}

export interface Diagnostics {
  databasePath: string | null;
  rootPath: string | null;
  thumbnailBytes: number;
  thumbnailLimitBytes: number;
  ffmpeg: boolean;
}

export interface Overview {
  problems: Problem[];
  report: Report;
  diagnostics: Diagnostics;
}

export type Tab = "items" | "report" | "diagnostics";

export const TABS: readonly { id: Tab; label: string }[] = [
  { id: "items", label: "Itens não reconhecidos" },
  { id: "report", label: "Relatório da indexação" },
  { id: "diagnostics", label: "Diagnóstico" },
];

export type StatusFilter = "all" | Status;

export const STATUS_LABEL: Record<Status, string> = { pending: "Pendente", resolved: "Resolvido", ignored: "Ignorado" };


/** "Todos · 37", "Pendentes · 9", … na ordem do canvas. */
export function statusFilters(problems: Problem[]): { id: StatusFilter; label: string }[] {
  const of = (status: Status) => problems.filter((problem) => problem.status === status).length;
  return [
    { id: "all", label: `Todos · ${formatCount(problems.length)}` },
    { id: "pending", label: `Pendentes · ${formatCount(of("pending"))}` },
    { id: "resolved", label: `Resolvidos · ${formatCount(of("resolved"))}` },
    { id: "ignored", label: `Ignorados · ${formatCount(of("ignored"))}` },
  ];
}

export function filterProblems(problems: Problem[], filter: StatusFilter): Problem[] {
  return filter === "all" ? problems : problems.filter((problem) => problem.status === filter);
}

/** `takeout-…/Takeout/Google Fotos/foto.json` → `…\Google Fotos\foto.json`, como no Windows. */
export function shortPath(path: string): string {
  const parts = path.split("/").filter((part) => part !== "");
  return parts.length <= 2 ? parts.join("\\") : `…\\${parts.slice(-2).join("\\")}`;
}

/** `E:\` + caminho relativo, para a dica ao passar o mouse. */
export function fullPath(root: string | null, path: string): string {
  const relative = path.replace(/\//g, "\\");
  return root === null ? relative : `${root.replace(/\\+$/, "")}\\${relative}`;
}

/** "6 min 41 s", "45 s", "1 h 02 min". */
export function durationText(seconds: number): string {
  if (seconds < 60) {
    return `${seconds} s`;
  }
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  if (hours > 0) {
    return `${hours} h ${String(minutes).padStart(2, "0")} min`;
  }
  return `${minutes} min ${seconds % 60} s`;
}

export function reportCards(report: Report): { label: string; value: string }[] {
  return [
    { label: "Mídias indexadas", value: formatCount(report.media) },
    { label: "Data pelo JSON do Takeout", value: formatCount(report.dateFromJson) },
    { label: "Data pelo EXIF", value: formatCount(report.dateFromExif) },
    { label: "Data pelo arquivo", value: formatCount(report.dateFromFile) },
    { label: "JSON em outra parte do export", value: formatCount(report.jsonInOtherPart) },
    { label: "Duração da última indexação", value: report.lastDurationSeconds === null ? "—" : durationText(report.lastDurationSeconds) },
  ];
}

export type Tone = "ok" | "warn" | "unknown";

/** `integrity`: `undefined` antes de verificar, `null` íntegro, texto com o problema. */
export function diagnosticLines(diagnostics: Diagnostics, integrity?: string | null): { label: string; value: string; tone: Tone }[] {
  const database = diagnostics.databasePath ?? "banco aberto";
  const health = integrity === undefined ? "não verificado" : integrity === null ? "íntegro" : `com defeito: ${integrity}`;
  const full = diagnostics.thumbnailBytes >= diagnostics.thumbnailLimitBytes;
  return [
    { label: "Banco de dados", value: `${database} · ${health}`, tone: integrity === undefined ? "unknown" : integrity === null ? "ok" : "warn" },
    {
      label: "Miniaturas em cache",
      value: `${fileSize(diagnostics.thumbnailBytes)} de ${fileSize(diagnostics.thumbnailLimitBytes)}`,
      tone: full ? "warn" : "ok",
    },
    {
      label: "ffmpeg",
      value: diagnostics.ffmpeg ? "disponível" : "não encontrado · vídeos sem capa nem duração",
      tone: diagnostics.ffmpeg ? "ok" : "warn",
    },
    { label: "Volume da biblioteca", value: `${diagnostics.rootPath ?? "?"} · conectado`, tone: "ok" },
    { label: "Conexões de rede", value: "nenhuma", tone: "ok" },
  ];
}
