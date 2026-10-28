// Configurações (F31, artboard `Settings`).
import { fileSize } from "../viewer/navigation.ts";

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
}

export const VIDEO_COVERS = "video_covers";
export const SCAN_ON_CONNECT = "scan_on_connect";
export const DEFAULT_CACHE_LIMIT = 4 * 1024 ** 3;

export const SECTIONS = [
  { id: "biblioteca", label: "Biblioteca" },
  { id: "dados", label: "Dados e cache" },
  { id: "backup", label: "Backup" },
  { id: "indexacao", label: "Indexação" },
  { id: "aparencia", label: "Aparência" },
  { id: "privacidade", label: "Privacidade" },
] as const;

/** "FOTOS · reconhecido pelo número de série, mesmo se a letra mudar". */
export function volumeLine(settings: Pick<Settings, "volumeLabel">): string {
  const label = settings.volumeLabel?.trim() || "Disco sem nome";
  return `${label} · reconhecido pelo número de série, mesmo se a letra mudar`;
}

/** "E:\.myphotos\library.db · 184 MB". */
export function databaseLine(settings: Pick<Settings, "databasePath" | "databaseBytes">): string {
  const path = settings.databasePath ?? "Banco sem arquivo";
  return settings.databaseBytes === null ? path : `${path} · ${fileSize(settings.databaseBytes)}`;
}

/** F48/F35: a lixeira do app, ao lado do banco: "E:\\.myphotos\\trash · 30 dias". */
export function trashLine(settings: Pick<Settings, "databasePath">): string {
  const path = settings.databasePath;
  if (path === null) {
    return "Banco sem arquivo";
  }
  const cut = Math.max(path.lastIndexOf("\\"), path.lastIndexOf("/"));
  const separator = path.includes("\\") ? "\\" : "/";
  return `${path.slice(0, cut)}${separator}trash · 30 dias`;
}

/** "1,2 GB usados de 4 GB" e quanto da barra pintar (0 a 1). */
export function cacheUsage(settings: Pick<Settings, "thumbnailBytes" | "thumbnailLimitBytes">): { text: string; fraction: number } {
  const limit = Math.max(1, settings.thumbnailLimitBytes);
  return {
    text: `${fileSize(settings.thumbnailBytes)} usados de ${fileSize(settings.thumbnailLimitBytes)}`,
    fraction: Math.min(1, Math.max(0, settings.thumbnailBytes / limit)),
  };
}

/** As opções do limite, com o valor atual mesmo se for de fora da lista. */
export function cacheOptions(settings: Pick<Settings, "cacheLimits" | "thumbnailLimitBytes">): { bytes: number; label: string }[] {
  const values = settings.cacheLimits.includes(settings.thumbnailLimitBytes)
    ? settings.cacheLimits
    : [...settings.cacheLimits, settings.thumbnailLimitBytes].sort((a, b) => a - b);
  return values.map((bytes) => ({ bytes, label: bytes === DEFAULT_CACHE_LIMIT ? `${fileSize(bytes)} (padrão)` : fileSize(bytes) }));
}

/** O que dizer sob "Gerar capa dos vídeos" quando ligar não basta. */
export function videoCoversNote(settings: Pick<Settings, "videoCovers" | "ffmpeg">): string | null {
  if (!settings.ffmpeg) {
    return "O ffmpeg não foi encontrado: os vídeos ficam com um ícone no lugar da capa.";
  }
  return settings.videoCovers ? null : "Os vídeos novos ficam com um ícone; as capas já feitas continuam.";
}

// Animações: seguir o Windows por padrão, ou fixas.

export type MotionChoice = "system" | "reduced" | "full";

export const MOTION_KEY = "myphotos.motion";

export const MOTION_OPTIONS: ReadonlyArray<{ choice: MotionChoice; label: string }> = [
  { choice: "system", label: "Seguir o Windows" },
  { choice: "reduced", label: "Sempre reduzidas" },
  { choice: "full", label: "Completas" },
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
