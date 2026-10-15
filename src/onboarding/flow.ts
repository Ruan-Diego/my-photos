// F16: quando mostrar as boas-vindas (artboard `Main`) e o texto da indexação em curso.

import type { DiskCard, LibrarySummary } from "../shell/library";

export interface LibraryFolder {
  rootPath: string;
  volumeLabel: string | null;
  takeoutExports: number;
}

export type Stage = "scanning" | "matching" | "resolving";

export interface Progress {
  stage: Stage;
  done: number;
  total: number;
}

export interface IndexingFinished {
  summary: unknown | null;
  error: string | null;
}

export const PROGRESS_EVENT = "indexing://progress";
export const FINISHED_EVENT = "indexing://finished";

/** Disco conectado e nunca indexado: primeiro uso. */
export function needsOnboarding(summary: LibrarySummary | null): boolean {
  return summary !== null && summary.connected && summary.lastIndexedAt === null && summary.mediaCount === 0;
}

const count = new Intl.NumberFormat("pt-BR");

/** "FOTOS · 3 exports do Takeout encontrados" */
export function folderDetail(folder: LibraryFolder): string {
  const exports =
    folder.takeoutExports === 0
      ? "nenhum export do Takeout na raiz"
      : folder.takeoutExports === 1
        ? "1 export do Takeout encontrado"
        : `${count.format(folder.takeoutExports)} exports do Takeout encontrados`;
  return folder.volumeLabel ? `${folder.volumeLabel} · ${exports}` : exports;
}

function stageText(progress: Progress): string {
  switch (progress.stage) {
    case "scanning":
      return "procurando arquivos";
    case "matching":
      return "lendo os JSONs do Takeout";
    case "resolving":
      return progress.total > 0
        ? `${count.format(progress.done)} de ${count.format(progress.total)} mídias`
        : "organizando as datas";
  }
}

/** Cartão da lateral enquanto a primeira indexação roda. */
export function indexingCard(root: string | null, progress: Progress): DiskCard {
  return {
    tone: "ok",
    title: root === null ? "Indexando" : `Indexando ${root}`,
    detail: stageText(progress),
    action: null,
  };
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
