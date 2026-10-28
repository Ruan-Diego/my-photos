// F28: a tela Problemas (artboard `Problems`): itens não reconhecidos, relatório da
// indexação e diagnóstico. Sem React.

import { fileSize } from "../viewer/navigation.ts";
import { formatCount, t } from "../i18n/i18n.ts";

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
  /** F37: a busca de atualizações, a única conexão de rede possível. */
  updateChecks?: boolean;
}

export interface Overview {
  problems: Problem[];
  report: Report;
  diagnostics: Diagnostics;
}

export type Tab = "items" | "report" | "diagnostics";

export const TABS: readonly { id: Tab; label: string }[] = [
  { id: "items", label: t("problems.tabItems") },
  { id: "report", label: t("problems.tabReport") },
  { id: "diagnostics", label: t("problems.tabDiagnostics") },
];

export type StatusFilter = "all" | Status;

export const STATUS_LABEL: Record<Status, string> = { pending: t("problems.pending"), resolved: t("problems.resolved"), ignored: t("problems.ignored") };

/** "Todos · 37", "Pendentes · 9", … na ordem do canvas. */
export function statusFilters(problems: Problem[]): { id: StatusFilter; label: string }[] {
  const of = (status: Status) => problems.filter((problem) => problem.status === status).length;
  return [
    { id: "all", label: t("problems.filterAll", { count: formatCount(problems.length) }) },
    { id: "pending", label: t("problems.filterPending", { count: formatCount(of("pending")) }) },
    { id: "resolved", label: t("problems.filterResolved", { count: formatCount(of("resolved")) }) },
    { id: "ignored", label: t("problems.filterIgnored", { count: formatCount(of("ignored")) }) },
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
    return t("problems.seconds", { seconds });
  }
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  if (hours > 0) {
    return t("problems.hoursMinutes", { hours, minutes: String(minutes).padStart(2, "0") });
  }
  return t("problems.minutesSeconds", { minutes, seconds: seconds % 60 });
}

export function reportCards(report: Report): { label: string; value: string }[] {
  return [
    { label: t("problems.reportMedia"), value: formatCount(report.media) },
    { label: t("indexing.countJson"), value: formatCount(report.dateFromJson) },
    { label: t("indexing.countExif"), value: formatCount(report.dateFromExif) },
    { label: t("problems.reportFile"), value: formatCount(report.dateFromFile) },
    { label: t("indexing.countOtherPart"), value: formatCount(report.jsonInOtherPart) },
    { label: t("problems.reportDuration"), value: report.lastDurationSeconds === null ? "—" : durationText(report.lastDurationSeconds) },
  ];
}

export type Tone = "ok" | "warn" | "unknown";

/** `integrity`: `undefined` antes de verificar, `null` íntegro, texto com o problema. */
export function diagnosticLines(diagnostics: Diagnostics, integrity?: string | null): { label: string; value: string; tone: Tone }[] {
  const database = diagnostics.databasePath ?? t("problems.databaseOpen");
  const health = integrity === undefined ? t("problems.unchecked") : integrity === null ? t("problems.sound") : t("problems.damaged", { detail: integrity });
  const full = diagnostics.thumbnailBytes >= diagnostics.thumbnailLimitBytes;
  return [
    { label: t("problems.database"), value: t("common.join", { a: database, b: health }), tone: integrity === undefined ? "unknown" : integrity === null ? "ok" : "warn" },
    {
      label: t("problems.thumbnails"),
      value: t("problems.cacheOf", { used: fileSize(diagnostics.thumbnailBytes), limit: fileSize(diagnostics.thumbnailLimitBytes) }),
      tone: full ? "warn" : "ok",
    },
    {
      label: "ffmpeg",
      value: diagnostics.ffmpeg ? t("problems.ffmpegOk") : t("problems.ffmpegMissing"),
      tone: diagnostics.ffmpeg ? "ok" : "warn",
    },
    { label: t("problems.volume"), value: t("problems.volumeConnected", { root: diagnostics.rootPath ?? "?" }), tone: "ok" },
    { label: t("problems.network"), value: diagnostics.updateChecks === true ? t("problems.networkUpdates") : t("problems.networkNone"), tone: "ok" },
  ];
}
