// F17: o que a tela de progresso (artboard `Indexing`) e o cartão da lateral mostram.

import type { DiskCard } from "../shell/library";
import { formatCount, list, t } from "../i18n/i18n.ts";

export type Stage = "scanning" | "hashing" | "matching" | "resolving" | "thumbnails";

export interface Counts {
  photos: number;
  videos: number;
  dateFromJson: number;
  dateFromExif: number;
  jsonInOtherPart: number;
  people: number;
  problems: number;
}

export interface Recent {
  kind: "paired" | "noJson";
  relativePath: string;
  jsonName: string | null;
  dateSource: string | null;
}

export interface Progress {
  stage: Stage;
  done: number;
  total: number;
  bytesDone: number;
  bytesTotal: number;
  counts: Counts;
  recent: Recent[];
}

export interface RecentThumbnail {
  mediaId: number;
  relativePath: string;
  src: string;
}

export const PROGRESS_EVENT = "indexing://progress";
export const FINISHED_EVENT = "indexing://finished";
/** Código do Rust quando o usuário pausou. */
export const PAUSED = "cancelled";

export const NO_COUNTS: Counts = {
  photos: 0,
  videos: 0,
  dateFromJson: 0,
  dateFromExif: 0,
  jsonInOtherPart: 0,
  people: 0,
  problems: 0,
};

/** Antes do primeiro evento. */
export function starting(): Progress {
  return { stage: "scanning", done: 0, total: 0, bytesDone: 0, bytesTotal: 0, counts: NO_COUNTS, recent: [] };
}

// As seis etapas do canvas. Leitura e pareamento saem da mesma passada do F08,
// metadados e gravação do mesmo lote do F10: andam juntas.
export const STEPS = list("indexing.steps");

const CURRENT_STEP: Record<Stage, number> = {
  scanning: 0,
  hashing: 0,
  matching: 1,
  resolving: 3,
  thumbnails: 5,
};

export type StepState = "done" | "current" | "todo";

export function stepStates(stage: Stage): StepState[] {
  const current = CURRENT_STEP[stage];
  return STEPS.map((_, index) => {
    if (index < current) {
      return "done";
    }
    if (index === current || (stage === "matching" && index === 2) || (stage === "resolving" && index === 4)) {
      return "current";
    }
    return "todo";
  });
}

// Peso de cada etapa no total, medido na primeira indexação real do `E:` (6.130 mídias,
// 109 GB): ler os arquivos domina. Estimativa, por isso "cerca de".
const WEIGHTS: Record<Stage, number> = {
  scanning: 0.02,
  hashing: 0.6,
  matching: 0.03,
  resolving: 0.15,
  thumbnails: 0.2,
};
const ORDER: Stage[] = ["scanning", "hashing", "matching", "resolving", "thumbnails"];

function stageFraction(progress: Progress): number {
  if (progress.stage === "hashing") {
    if (progress.bytesTotal > 0) {
      return progress.bytesDone / progress.bytesTotal;
    }
  }
  if (progress.stage === "scanning" || progress.stage === "matching") {
    return 0;
  }
  return progress.total > 0 ? progress.done / progress.total : 0;
}

/** 0..1 da indexação inteira. */
export function overall(progress: Progress): number {
  const index = ORDER.indexOf(progress.stage);
  const before = ORDER.slice(0, index).reduce((sum, stage) => sum + WEIGHTS[stage], 0);
  const fraction = before + WEIGHTS[progress.stage] * Math.min(1, Math.max(0, stageFraction(progress)));
  return Math.min(1, fraction);
}

/** "cerca de 4 min restantes", pelo ritmo desde o início; `null` cedo demais para dizer. */
export function remaining(fraction: number, elapsedMs: number): string | null {
  if (fraction < 0.02 || elapsedMs < 10_000 || fraction >= 1) {
    return null;
  }
  const left = (elapsedMs * (1 - fraction)) / fraction;
  const minutes = Math.round(left / 60_000);
  if (minutes < 1) {
    return t("indexing.lessThanMinute");
  }
  if (minutes < 60) {
    return t("indexing.aboutMinutes", { minutes });
  }
  const hours = Math.floor(minutes / 60);
  const rest = minutes % 60;
  return rest === 0 ? t("indexing.aboutHours", { hours }) : t("indexing.aboutHoursMinutes", { hours, minutes: rest });
}

/** "62% · cerca de 4 min restantes" */
export function percentLine(fraction: number, elapsedMs: number): string {
  const percent = `${Math.floor(fraction * 100)}%`;
  const left = remaining(fraction, elapsedMs);
  return left === null ? percent : `${percent} · ${left}`;
}

/** O título grande: "2.633 de 4.247 mídias". */
export function headline(progress: Progress): string {
  switch (progress.stage) {
    case "scanning":
      return t("indexing.scanning");
    case "matching":
      return t("indexing.matching");
    case "hashing":
      return t("indexing.hashing", { done: formatCount(progress.done), total: formatCount(progress.total) });
    case "resolving":
      return t("indexing.resolving", { done: formatCount(progress.done), total: formatCount(progress.total) });
    case "thumbnails":
      return t("indexing.thumbnails", { done: formatCount(progress.done), total: formatCount(progress.total) });
  }
}

export interface CountCard {
  label: string;
  value: string;
}

export function countCards(counts: Counts): CountCard[] {
  return [
    { label: t("indexing.countPhotos"), value: formatCount(counts.photos) },
    { label: t("indexing.countVideos"), value: formatCount(counts.videos) },
    { label: t("indexing.countJson"), value: formatCount(counts.dateFromJson) },
    { label: t("indexing.countExif"), value: formatCount(counts.dateFromExif) },
    { label: t("indexing.countOtherPart"), value: formatCount(counts.jsonInOtherPart) },
    { label: t("indexing.countPeople"), value: formatCount(counts.people) },
  ];
}

/** "12 itens não reconhecidos até agora — a indexação continua"; `null` sem problemas. */
export function problemsLine(problems: number, paused: boolean): string | null {
  if (problems === 0) {
    return null;
  }
  const items = t("indexing.unrecognized", { n: problems });
  return paused ? t("indexing.soFar", { items }) : t("indexing.soFarContinues", { items });
}

/** `Viagem Europa 2022\foto001.jpg`: a pasta e o arquivo, como no Windows. */
export function shortPath(relativePath: string): string {
  return relativePath.split("/").slice(-2).join("\\");
}

export interface LogLine {
  text: string;
  /** Sem JSON: destacado no console. */
  warn: boolean;
}

const DATE_FROM: Record<string, () => string> = {
  exif: () => t("indexing.dateFromExif"),
  filesystem: () => t("indexing.dateFromFile"),
};

export function logLine(recent: Recent): LogLine {
  if (recent.kind === "paired") {
    return { text: t("indexing.paired", { path: shortPath(recent.relativePath), json: recent.jsonName ?? "JSON" }), warn: false };
  }
  const date = recent.dateSource === null ? t("indexing.noDate") : (DATE_FROM[recent.dateSource]?.() ?? recent.dateSource);
  return { text: t("indexing.noJson", { path: shortPath(recent.relativePath), date }), warn: true };
}

export function thumbnailLine(thumbnail: RecentThumbnail): LogLine {
  return { text: t("indexing.thumbnailLine", { path: shortPath(thumbnail.relativePath) }), warn: false };
}

/** As últimas `keep` linhas, a mais nova embaixo. */
export function pushLines(lines: LogLine[], more: LogLine[], keep = 4): LogLine[] {
  return [...lines, ...more].slice(-keep);
}

function drive(root: string | null): string {
  return root ?? t("indexing.theLibrary");
}

/** Rótulo acima do título. */
export function label(root: string | null, paused: boolean): string {
  return paused ? t("indexing.pausedLabel", { drive: drive(root) }) : t("indexing.indexingLabel", { drive: drive(root) });
}

/** Cartão da lateral enquanto a indexação roda ou está pausada. */
export function indexingCard(root: string | null, progress: Progress, paused: boolean): DiskCard {
  const percent = `${Math.floor(overall(progress) * 100)}%`;
  if (paused) {
    return { tone: "warn", title: t("indexing.pausedTitle"), detail: t("indexing.percentDone", { percent }), action: t("indexing.continue") };
  }
  return {
    tone: "ok",
    title: t("indexing.indexingLabel", { drive: drive(root) }),
    detail: t("common.join", { a: percent, b: headline(progress).replace("…", "").toLowerCase() }),
    action: t("indexing.seeProgress"),
  };
}

/** Disco com mídias mas sem indexação completa: uma primeira indexação foi pausada ou parou. */
export function interrupted(summary: { connected: boolean; lastIndexedAt: string | null; mediaCount: number } | null): boolean {
  return summary !== null && summary.connected && summary.lastIndexedAt === null && summary.mediaCount > 0;
}

export function interruptedCard(mediaCount: number): DiskCard {
  return {
    tone: "warn",
    title: t("indexing.incomplete"),
    detail: t("indexing.soFarMedia", { media: t("count.media", { n: mediaCount }) }),
    action: t("indexing.continue"),
  };
}

/** Índice completo, mas o app fechou (ou pausou) no meio das miniaturas: quantas faltam. */
export function missingThumbnails(
  summary: { connected: boolean; lastIndexedAt: string | null; missingThumbnails?: number } | null,
): number {
  if (summary === null || !summary.connected || summary.lastIndexedAt === null) {
    return 0;
  }
  return summary.missingThumbnails ?? 0;
}

export function thumbnailsCard(missing: number): DiskCard {
  return {
    tone: "warn",
    title: t("indexing.incomplete"),
    detail: t("count.thumbnailsLeft", { n: missing }),
    action: t("indexing.continue"),
  };
}
