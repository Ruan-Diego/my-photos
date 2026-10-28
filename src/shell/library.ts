// Cartão do disco no rodapé da barra lateral (artboard `Sidebar`).

import { currentLocale, t } from "../i18n/i18n.ts";

export interface LibrarySummary {
  connected: boolean;
  rootPath: string | null;
  volumeLabel: string | null;
  mediaCount: number;
  lastIndexedAt: string | null;
  /** Mídias ainda sem miniatura pequena (o app fechou durante as miniaturas). */
  missingThumbnails: number;
  problem: string | null;
}

export interface DiskCard {
  tone: "ok" | "warn";
  title: string;
  detail: string;
  /** Link no rodapé do cartão; `null` quando não há o que fazer. */
  action: string | null;
}

function clock(iso: string, now: Date): string | null {
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) {
    return null;
  }
  const time = date.toLocaleTimeString(currentLocale(), { hour: "2-digit", minute: "2-digit" });
  if (date.toDateString() === now.toDateString()) {
    return t("disk.updatedAt", { time });
  }
  const day = date.toLocaleDateString(currentLocale(), { day: "numeric", month: "short" });
  return t("disk.updatedOn", { day: day.replace(".", "") });
}

/** `null` summary: the command itself failed (the app could not ask). */
export function diskCard(summary: LibrarySummary | null, now: Date = new Date()): DiskCard {
  if (summary === null || !summary.connected) {
    return {
      tone: "warn",
      title: t("disk.disconnected"),
      detail: summary?.problem ?? t("disk.unreadable"),
      action: t("disk.searchAgain"),
    };
  }
  const media = t("count.media", { n: summary.mediaCount });
  const when = summary.lastIndexedAt === null ? t("disk.neverIndexed") : clock(summary.lastIndexedAt, now);
  return {
    tone: "ok",
    title: summary.rootPath === null ? t("disk.connected") : t("disk.rootConnected", { root: summary.rootPath }),
    detail: when === null ? media : `${media} · ${when}`,
    action: t("disk.scanNew"),
  };
}
