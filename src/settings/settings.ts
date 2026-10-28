// Configurações (F31, artboard `Settings`).
import { fileSize } from "../viewer/navigation.ts";
import { t } from "../i18n/i18n.ts";

export interface Settings {
  rootPath: string | null;
  volumeLabel: string | null;
  databasePath: string | null;
  databaseBytes: number | null;
  thumbnailBytes: number;
  thumbnailLimitBytes: number;
  cacheLimits: number[];
  videoCovers: boolean;
  ffmpeg: boolean;
  scanOnConnect: boolean;
  ignoredFolders: string[];
  /** F37: "Procurar atualizações ao abrir" (desligado por padrão: nada de rede, F36). */
  updateChecks: boolean;
  /** Build de release, com a chave e o endereço do updater. */
  updaterConfigured: boolean;
  version: string;
}

/** O que `app_update_check` devolve quando há versão nova. */
export interface UpdateAvailable {
  version: string;
  notes: string | null;
}

export const VIDEO_COVERS = "video_covers";
export const SCAN_ON_CONNECT = "scan_on_connect";
export const UPDATE_CHECKS = "update_checks";
export const DEFAULT_CACHE_LIMIT = 4 * 1024 ** 3;

export const SECTIONS = [
  { id: "biblioteca", label: t("settings.sections.library") },
  { id: "dados", label: t("settings.sections.data") },
  { id: "backup", label: t("settings.sections.backup") },
  { id: "indexacao", label: t("settings.sections.indexing") },
  { id: "aparencia", label: t("settings.sections.appearance") },
  { id: "privacidade", label: t("settings.sections.privacy") },
] as const;

/** "FOTOS · reconhecido pelo número de série, mesmo se a letra mudar". */
export function volumeLine(settings: Pick<Settings, "volumeLabel">): string {
  const label = settings.volumeLabel?.trim() || t("settings.unnamedDisk");
  return t("settings.volumeLine", { label });
}

/** "E:\.myphotos\library.db · 184 MB". */
export function databaseLine(settings: Pick<Settings, "databasePath" | "databaseBytes">): string {
  const path = settings.databasePath ?? t("settings.noDatabaseFile");
  return settings.databaseBytes === null ? path : t("common.join", { a: path, b: fileSize(settings.databaseBytes) });
}

/** F48/F35: a lixeira do app, ao lado do banco: "E:\\.myphotos\\trash · 30 dias". */
export function trashLine(settings: Pick<Settings, "databasePath">): string {
  const path = settings.databasePath;
  if (path === null) {
    return t("settings.noDatabaseFile");
  }
  const cut = Math.max(path.lastIndexOf("\\"), path.lastIndexOf("/"));
  const separator = path.includes("\\") ? "\\" : "/";
  return t("settings.trashLine", { path: `${path.slice(0, cut)}${separator}trash` });
}

/** "1,2 GB usados de 4 GB" e quanto da barra pintar (0 a 1). */
export function cacheUsage(settings: Pick<Settings, "thumbnailBytes" | "thumbnailLimitBytes">): { text: string; fraction: number } {
  const limit = Math.max(1, settings.thumbnailLimitBytes);
  return {
    text: t("settings.cacheUsage", { used: fileSize(settings.thumbnailBytes), limit: fileSize(settings.thumbnailLimitBytes) }),
    fraction: Math.min(1, Math.max(0, settings.thumbnailBytes / limit)),
  };
}

/** As opções do limite, com o valor atual mesmo se for de fora da lista. */
export function cacheOptions(settings: Pick<Settings, "cacheLimits" | "thumbnailLimitBytes">): { bytes: number; label: string }[] {
  const values = settings.cacheLimits.includes(settings.thumbnailLimitBytes)
    ? settings.cacheLimits
    : [...settings.cacheLimits, settings.thumbnailLimitBytes].sort((a, b) => a - b);
  return values.map((bytes) => ({ bytes, label: bytes === DEFAULT_CACHE_LIMIT ? t("settings.defaultOption", { value: fileSize(bytes) }) : fileSize(bytes) }));
}

/** O que dizer sob "Gerar capa dos vídeos" quando ligar não basta. */
export function videoCoversNote(settings: Pick<Settings, "videoCovers" | "ffmpeg">): string | null {
  if (!settings.ffmpeg) {
    return t("settings.noFfmpeg");
  }
  return settings.videoCovers ? null : t("settings.coversOff");
}

// Animações: seguir o Windows por padrão, ou fixas.

export type MotionChoice = "system" | "reduced" | "full";

export const MOTION_KEY = "myphotos.motion";

export const MOTION_OPTIONS: ReadonlyArray<{ choice: MotionChoice; label: string }> = [
  { choice: "system", label: t("settings.followWindows") },
  { choice: "reduced", label: t("settings.motionReduced") },
  { choice: "full", label: t("settings.motionFull") },
];

export function parseMotion(stored: string | null | undefined): MotionChoice {
  return stored === "reduced" || stored === "full" ? stored : "system";
}

/** Valor do `reducedMotion` do `MotionConfig`. */
export function reducedMotion(choice: MotionChoice): "user" | "always" | "never" {
  return choice === "reduced" ? "always" : choice === "full" ? "never" : "user";
}

export function loadMotion(): MotionChoice {
  try {
    return parseMotion(globalThis.localStorage?.getItem(MOTION_KEY));
  } catch {
    return "system";
  }
}

export function saveMotion(choice: MotionChoice): void {
  try {
    if (choice === "system") {
      globalThis.localStorage?.removeItem(MOTION_KEY);
    } else {
      globalThis.localStorage?.setItem(MOTION_KEY, choice);
    }
  } catch {
    // Sem armazenamento a escolha vale até fechar o app.
  }
}

/** F36/F37: o que o cartão Privacidade diz, conforme a busca de atualizações. */
export function privacyNote(settings: Pick<Settings, "updateChecks"> | null): string {
  return settings?.updateChecks === true ? t("settings.privacyUpdates") : t("settings.privacyNote");
}
