// Pastas e lixeira (F48, artboards `Folders` e `Trash`; AD-008, AD-009).

export interface Folder {
  /** `""` é a raiz; senão `a/b`, com `/`. */
  path: string;
  name: string;
  depth: number;
  direct: number;
  total: number;
}

export interface FolderItem {
  id: number;
  filename: string;
  video: boolean;
  hasJson: boolean;
}

export interface Outcome {
  done: number;
  skipped: [string, string][];
}

export interface TrashItem {
  id: number;
  kind: "file" | "folder";
  mediaId: number | null;
  name: string;
  folder: string;
  items: number;
  trashedAt: string;
  purgeAfter: string;
}

export const FOLDERS_HASH = "#/pastas";
export const TRASH_HASH = "#/lixeira";

/** `#/pastas/Takeout%2FViagem`. */
export function folderHash(path: string): string {
  return path === "" ? FOLDERS_HASH : `${FOLDERS_HASH}/${encodeURIComponent(path)}`;
}

export function folderFromHash(hash: string): string {
  const match = /^#\/pastas\/(.+)$/.exec(hash);
  if (match === null) {
    return "";
  }
  try {
    return decodeURIComponent(match[1]);
  } catch {
    return "";
  }
}

/** A pasta de um caminho de mídia (`a/b/c.jpg` → `a/b`). */
export function parentOf(path: string): string {
  const at = path.lastIndexOf("/");
  return at < 0 ? "" : path.slice(0, at);
}

/** `E:\ › Takeout › Viagem`: cada parte com o caminho para onde leva. A última é a atual. */
export function breadcrumb(path: string, rootLabel: string): { label: string; path: string }[] {
  const parts = path === "" ? [] : path.split("/");
  return [{ label: rootLabel, path: "" }, ...parts.map((label, at) => ({ label, path: parts.slice(0, at + 1).join("/") }))];
}

/** Nome mostrado de uma pasta, com a raiz pelo rótulo do disco. */
export function folderTitle(path: string, rootLabel: string): string {
  return path === "" ? rootLabel : path.slice(path.lastIndexOf("/") + 1);
}

/** Destino no diálogo "Mover para…": `Organizadas \ Portugal`. */
export function targetLabel(path: string, rootLabel: string): string {
  return path === "" ? rootLabel : path.split("/").join(" \\ ");
}

/** Clique liga ou desliga; com Shift, marca do último clicado até este. */
export function select(selected: ReadonlySet<number>, order: readonly number[], id: number, anchor: number | null, range: boolean): Set<number> {
  const next = new Set(selected);
  if (range && anchor !== null) {
    const from = order.indexOf(anchor);
    const to = order.indexOf(id);
    if (from >= 0 && to >= 0) {
      const [start, end] = from < to ? [from, to] : [to, from];
      for (const each of order.slice(start, end + 1)) {
        next.add(each);
      }
      return next;
    }
  }
  if (next.has(id)) {
    next.delete(id);
  } else {
    next.add(id);
  }
  return next;
}

const count = new Intl.NumberFormat("pt-BR");

export function itemsLabel(n: number): string {
  return n === 1 ? "1 item" : `${count.format(n)} itens`;
}

/** "124 itens nesta pasta · 118 com JSON do Takeout". */
export function folderSummary(items: readonly FolderItem[]): string {
  if (items.length === 0) {
    return "Nenhuma foto ou vídeo direto nesta pasta";
  }
  const json = items.filter((item) => item.hasJson).length;
  return `${itemsLabel(items.length)} nesta pasta · ${count.format(json)} com JSON do Takeout`;
}

/** O que dizer depois de mover, excluir ou restaurar. */
export function outcomeText(outcome: Outcome, verb: string, tail = ""): string {
  const done = outcome.done === 1 ? `1 item ${verb}${tail}` : `${count.format(outcome.done)} itens ${verb}s${tail}`;
  if (outcome.skipped.length === 0) {
    return `${done}.`;
  }
  const [name, reason] = outcome.skipped[0];
  const more = outcome.skipped.length > 1 ? ` e mais ${outcome.skipped.length - 1}` : "";
  return `${done}. Ficou como estava: ${name} (${reason})${more}.`;
}

const DAY = 24 * 60 * 60 * 1000;

/** "apagado em 29 dias"; `warn` nos últimos 3 dias. */
export function purgeText(purgeAfter: string, now: number): { text: string; warn: boolean } {
  const left = Math.ceil((Date.parse(purgeAfter) - now) / DAY);
  const text = left <= 0 ? "apagado hoje" : left === 1 ? "apagado amanhã" : `apagado em ${left} dias`;
  return { text, warn: left <= 3 };
}

/** `Pasta “Rascunhos” · 42 itens` ou o nome do arquivo. */
export function trashName(item: TrashItem): string {
  return item.kind === "folder" ? `Pasta “${item.name}” · ${itemsLabel(item.items)}` : item.name;
}

/** "Estava em Google Fotos\Viagem", ou na raiz do disco. */
export function trashOrigin(item: TrashItem, rootLabel: string): string {
  return `Estava em ${item.folder === "" ? rootLabel : item.folder.split("/").join("\\")}`;
}
