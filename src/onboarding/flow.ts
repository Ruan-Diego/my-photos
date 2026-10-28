// F16: quando mostrar as boas-vindas (artboard `Main`) e o cartão de uma indexação que falhou.

import type { DiskCard, LibrarySummary } from "../shell/library";
import { maybe, t } from "../i18n/i18n.ts";

export interface LibraryFolder {
  rootPath: string;
  volumeLabel: string | null;
  takeoutExports: number;
}

export interface IndexingFinished {
  summary: unknown | null;
  error: string | null;
}

/** Disco conectado e nunca indexado: primeiro uso. */
export function needsOnboarding(summary: LibrarySummary | null): boolean {
  return summary !== null && summary.connected && summary.lastIndexedAt === null && summary.mediaCount === 0;
}

/** "FOTOS · 3 exports do Takeout encontrados" */
export function folderDetail(folder: LibraryFolder): string {
  const exports = folder.takeoutExports === 0 ? t("onboarding.noExports") : t("onboarding.exports", { n: folder.takeoutExports });
  return folder.volumeLabel ? `${folder.volumeLabel} · ${exports}` : exports;
}

// Códigos de F05 (ScanError) e F04 (DbError), com as mesmas frases do Rust.
export function failureText(code: string): string {
  return (/^\w+$/.test(code) ? maybe(`onboarding.failures.${code}`) : undefined) ?? t("onboarding.unknownFailure", { code });
}

export function failedCard(code: string): DiskCard {
  return {
    tone: "warn",
    title: t("onboarding.stoppedTitle"),
    detail: failureText(code),
    action: t("common.retry"),
  };
}
