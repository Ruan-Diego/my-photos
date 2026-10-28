// F16: quando mostrar as boas-vindas (artboard `Main`) e o cartão de uma indexação que falhou.

import type { DiskCard, LibrarySummary } from "../shell/library";
import { formatCount } from "../i18n/i18n.ts";

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
  const exports =
    folder.takeoutExports === 0
      ? "nenhum export do Takeout encontrado"
      : folder.takeoutExports === 1
        ? "1 export do Takeout encontrado"
        : `${formatCount(folder.takeoutExports)} exports do Takeout encontrados`;
  return folder.volumeLabel ? `${folder.volumeLabel} · ${exports}` : exports;
}

// Códigos de F05 (ScanError) e F04 (DbError), com as mesmas frases do Rust.
const FAILURES: Record<string, string> = {
  volume_missing: "disco da biblioteca não encontrado",
  root_missing: "pasta da biblioteca não encontrada",
  root_not_directory: "a biblioteca não é uma pasta",
  db_write_failed: "falha ao gravar no banco",
  open_failed: "não foi possível abrir o banco",
  db_newer_than_app: "o banco é de uma versão mais nova do My Photos",
  migration_failed: "falha ao atualizar o banco",
};

export function failureText(code: string): string {
  return FAILURES[code] ?? `erro: ${code}`;
}

export function failedCard(code: string): DiskCard {
  return {
    tone: "warn",
    title: "A indexação parou",
    detail: failureText(code),
    action: "Tentar de novo",
  };
}
