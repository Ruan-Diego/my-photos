// Cartão do disco no rodapé da barra lateral (artboard `Sidebar`).
import { dateText, t } from "../i18n/i18n.ts";

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
  const time = dateText(date, { hour: "2-digit", minute: "2-digit" });
  if (date.toDateString() === now.toDateString()) {
    return `atualizado às ${time}`;
  }
  const day = dateText(date, { day: "numeric", month: "short" });
  return `atualizado em ${day.replace(".", "")}`;
}

/** `null` summary: the command itself failed (the app could not ask). */
export function diskCard(summary: LibrarySummary | null, now: Date = new Date()): DiskCard {
  if (summary === null || !summary.connected) {
    return {
      tone: "warn",
      title: "HD desconectado",
      detail: summary?.problem ?? "não foi possível ler a biblioteca",
      action: "Procurar de novo",
    };
  }
  const media = t("count.media", { n: summary.mediaCount });
  const when = summary.lastIndexedAt === null ? "ainda não indexado" : clock(summary.lastIndexedAt, now);
  return {
    tone: "ok",
    title: summary.rootPath === null ? "Biblioteca conectada" : `${summary.rootPath} conectado`,
    detail: when === null ? media : `${media} · ${when}`,
    action: "Procurar arquivos novos",
  };
}
