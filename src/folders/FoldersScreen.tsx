import { useCallback, useEffect, useMemo, useRef, useState, type FormEvent } from "react";
import { invoke } from "@tauri-apps/api/core";
import { thumbnailUrl } from "../viewer/media";
import {
  breadcrumb,
  folderFromHash,
  folderHash,
  folderSummary,
  folderTitle,
  outcomeText,
  select,
  targetLabel,
  type Folder,
  type FolderItem,
  type Outcome,
} from "./folders";

type Dialog = "create" | "rename" | "move" | "trash-folder" | null;

const folderIcon = "M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z";
const driveIcon = "M3 8h18v8H3zM7 12h.01";
const stroke = { fill: "none", stroke: "currentColor", strokeWidth: 1.8, strokeLinecap: "round", strokeLinejoin: "round" } as const;

function useFolderHash(): string {
  const [folder, setFolder] = useState(() => folderFromHash(window.location.hash));
  useEffect(() => {
    const follow = () => setFolder(folderFromHash(window.location.hash));
    window.addEventListener("hashchange", follow);
    return () => window.removeEventListener("hashchange", follow);
  }, []);
  return folder;
}

function errorText(error: unknown): string {
  return typeof error === "string" ? error : error instanceof Error ? error.message : String(error);
}

/** Pastas como estão no disco, com mover, renomear, criar e lixeira (F48, artboard `Folders`). */
export function FoldersScreen({ rootLabel, writable, onChanged }: { rootLabel: string; writable: boolean; onChanged: () => void }) {
  const current = useFolderHash();
  const [tree, setTree] = useState<Folder[] | null>(null);
  const [items, setItems] = useState<FolderItem[] | null>(null);
  const [failure, setFailure] = useState<string | null>(null);
  const [selected, setSelected] = useState<Set<number>>(new Set());
  const [anchor, setAnchor] = useState<number | null>(null);
  const [dialog, setDialog] = useState<Dialog>(null);
  const [notice, setNotice] = useState<{ text: string; warn: boolean } | null>(null);
  const [busy, setBusy] = useState(false);
  const [version, setVersion] = useState(0);

  useEffect(() => {
    invoke<Folder[]>("folder_tree")
      .then((value) => {
        setTree(value);
        setFailure(null);
      })
      .catch((error) => setFailure(errorText(error)));
  }, [version]);

  useEffect(() => {
    let live = true;
    setItems(null);
    invoke<FolderItem[]>("folder_items", { folder: current })
      .then((value) => live && setItems(value))
      .catch((error) => live && setFailure(errorText(error)));
    return () => {
      live = false;
    };
  }, [current, version]);

  useEffect(() => {
    setSelected(new Set());
    setAnchor(null);
    setDialog(null);
  }, [current]);

  const known = tree === null || tree.some((folder) => folder.path.toLowerCase() === current.toLowerCase());
  const order = useMemo(() => (items ?? []).map((item) => item.id), [items]);

  // Toda escrita termina igual: relê a árvore e a pasta, limpa a seleção e avisa o app.
  const run = useCallback(
    <T,>(work: Promise<T>, done: (value: T) => string, after?: (value: T) => void) => {
      setBusy(true);
      work
        .then((value) => {
          setNotice({ text: done(value), warn: false });
          setSelected(new Set());
          setDialog(null);
          after?.(value);
          onChanged();
        })
        .catch((error) => setNotice({ text: errorText(error), warn: true }))
        .finally(() => {
          setBusy(false);
          setVersion((value) => value + 1);
        });
    },
    [onChanged],
  );

  const ids = [...selected];

  const crumbs = breadcrumb(current, rootLabel);
  const title = folderTitle(current, rootLabel);

  return (
    <main className="page folders" aria-labelledby="page-title">
      <header className="page-header folders-header">
        <div className="folders-title">
          {crumbs.length > 1 ? (
            <nav aria-label="Caminho" className="folders-crumbs">
              {crumbs.slice(0, -1).map((crumb) => (
                <span key={crumb.path}>
                  <a href={folderHash(crumb.path)}>{crumb.label}</a>
                  <span aria-hidden="true">›</span>
                </span>
              ))}
            </nav>
          ) : null}
          <h1 id="page-title">{current === "" ? "Pastas" : title}</h1>
        </div>
        <button type="button" className="button-line folders-action" disabled={!writable || !known} onClick={() => setDialog("create")}>
          <svg width="18" height="18" viewBox="0 0 24 24" {...stroke} aria-hidden="true">
            <path d={`${folderIcon}M12 10v6M9 13h6`} />
          </svg>
          Nova pasta
        </button>
        <button type="button" className="button-line folders-action" disabled={!writable || current === "" || !known} onClick={() => setDialog("rename")}>
          <svg width="18" height="18" viewBox="0 0 24 24" {...stroke} aria-hidden="true">
            <path d="M4 20h4L19 9l-4-4L4 16zM13 7l4 4" />
          </svg>
          Renomear pasta
        </button>
        <button
          type="button"
          className="button-line folders-trash"
          aria-label="Mover pasta para a lixeira"
          title="Mover pasta para a lixeira"
          disabled={!writable || current === "" || !known}
          onClick={() => setDialog("trash-folder")}
        >
          <svg width="20" height="20" viewBox="0 0 24 24" {...stroke} aria-hidden="true">
            <path d="M4 7h16M6 7l1 13h10l1-13M9 7V4h6v3" />
          </svg>
        </button>
      </header>

      <div className="folders-body">
        <nav aria-label="Árvore de pastas" className="folders-tree">
          {(tree ?? []).map((folder) => {
            const here = folder.path.toLowerCase() === current.toLowerCase();
            return (
              <a
                key={folder.path}
                href={folderHash(folder.path)}
                aria-current={here ? "page" : undefined}
                style={{ paddingLeft: 10 + folder.depth * 16 }}
              >
                <svg width="16" height="16" viewBox="0 0 24 24" {...stroke} aria-hidden="true">
                  <path d={folder.path === "" ? driveIcon : folderIcon} />
                </svg>
                <span className="folders-tree-name">{folder.path === "" ? rootLabel : folder.name}</span>
                <span className="folders-tree-count">{folder.total > 0 ? folder.total.toLocaleString("pt-BR") : ""}</span>
              </a>
            );
          })}
        </nav>

        <section className="folders-content" aria-label={`Conteúdo de ${title}`}>
          {notice !== null ? (
            <p className={notice.warn ? "folders-notice folders-notice-warn" : "folders-notice"} role="status">
              {notice.text}
              <button type="button" aria-label="Fechar aviso" onClick={() => setNotice(null)}>
                ×
              </button>
            </p>
          ) : null}
          {failure !== null ? (
            <div className="page-empty">
              <p className="page-empty-title">Não foi possível ler as pastas</p>
              <p className="page-empty-detail">{failure}</p>
            </div>
          ) : !known ? (
            <div className="page-empty">
              <p className="page-empty-title">Esta pasta não existe mais</p>
              <p className="page-empty-detail">
                Ela foi renomeada, movida ou excluída. <a href={folderHash("")}>Voltar para {rootLabel}</a>
              </p>
            </div>
          ) : (
            <>
              <div className="folders-meta">
                <span>{items === null ? " " : folderSummary(items)}</span>
                {items !== null && items.length > 0 ? <span>Clique para selecionar · Shift para intervalo</span> : null}
              </div>
              <div className="folders-grid">
                {(items ?? []).map((item) => {
                  const on = selected.has(item.id);
                  return (
                    <button
                      key={item.id}
                      type="button"
                      className="folders-tile"
                      aria-pressed={on}
                      aria-label={item.filename}
                      title={item.filename}
                      onClick={(event) => {
                        setSelected(select(selected, order, item.id, anchor, event.shiftKey));
                        setAnchor(item.id);
                      }}
                    >
                      <img src={thumbnailUrl(item.id)} alt="" loading="lazy" draggable={false} />
                      <span className="folders-check" aria-hidden="true">
                        {on ? (
                          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="#fff" strokeWidth="3.5" strokeLinecap="round" strokeLinejoin="round">
                            <path d="M5 12l5 5 9-10" />
                          </svg>
                        ) : null}
                      </span>
                      {item.video ? <span className="folders-video">Vídeo</span> : null}
                    </button>
                  );
                })}
              </div>
            </>
          )}

          {selected.size > 0 ? (
            <div role="toolbar" aria-label="Ações da seleção" className="folders-toolbar">
              <strong>{selected.size === 1 ? "1 selecionada" : `${selected.size} selecionadas`}</strong>
              <button type="button" className="folders-toolbar-main" disabled={!writable || busy} onClick={() => setDialog("move")}>
                <svg width="18" height="18" viewBox="0 0 24 24" {...stroke} aria-hidden="true">
                  <path d={`${folderIcon}M10 13h6M13 10l3 3-3 3`} />
                </svg>
                Mover para…
              </button>
              <button
                type="button"
                disabled={!writable || busy}
                onClick={() => run(invoke<Outcome>("media_trash", { ids }), (outcome) => outcomeText(outcome, "movido", " para a lixeira, onde fica 30 dias"))}
              >
                <svg width="18" height="18" viewBox="0 0 24 24" {...stroke} aria-hidden="true">
                  <path d="M4 7h16M6 7l1 13h10l1-13M9 7V4h6v3" />
                </svg>
                Mover para a lixeira
              </button>
              <button type="button" aria-label="Limpar seleção" className="folders-toolbar-close" onClick={() => setSelected(new Set())}>
                <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round" aria-hidden="true">
                  <path d="M6 6l12 12M18 6L6 18" />
                </svg>
              </button>
            </div>
          ) : null}

          {dialog === "move" ? (
            <MoveDialog
              tree={tree ?? []}
              current={current}
              count={selected.size}
              rootLabel={rootLabel}
              busy={busy}
              onCancel={() => setDialog(null)}
              onMove={(target) =>
                run(invoke<Outcome>("media_move", { ids, folder: target }), (outcome) => `${outcomeText(outcome, "movido")} Destino: ${targetLabel(target, rootLabel)}.`)
              }
            />
          ) : dialog === "create" || dialog === "rename" ? (
            <NameDialog
              title={dialog === "create" ? `Nova pasta em ${title}` : `Renomear ${title}`}
              action={dialog === "create" ? "Criar" : "Renomear"}
              initial={dialog === "create" ? "" : title}
              busy={busy}
              onCancel={() => setDialog(null)}
              onSubmit={(name) =>
                dialog === "create"
                  ? run(invoke<string>("folder_create", { parent: current, name }), () => `Pasta ${name.trim()} criada.`, (path) => {
                      window.location.hash = folderHash(path);
                    })
                  : run(invoke<string>("folder_rename", { folder: current, name }), () => `Pasta renomeada para ${name.trim()}.`, (path) => {
                      window.location.hash = folderHash(path);
                    })
              }
            />
          ) : dialog === "trash-folder" ? (
            <div role="dialog" aria-label="Mover pasta para a lixeira" className="folders-dialog">
              <h2>Mover {title} para a lixeira?</h2>
              <p className="folders-dialog-note">
                A pasta vai inteira, com as subpastas e os JSONs do Takeout. Ela fica 30 dias na lixeira e pode ser restaurada de lá.
              </p>
              <div className="folders-dialog-actions">
                <button type="button" className="button-line" disabled={busy} onClick={() => setDialog(null)}>
                  Cancelar
                </button>
                <button
                  type="button"
                  className="button-solid"
                  disabled={busy}
                  onClick={() =>
                    run(invoke("folder_trash", { folder: current }), () => `${title} foi para a lixeira, onde fica 30 dias.`, () => {
                      window.location.hash = folderHash(current.includes("/") ? current.slice(0, current.lastIndexOf("/")) : "");
                    })
                  }
                >
                  Mover para a lixeira
                </button>
              </div>
            </div>
          ) : null}
        </section>
      </div>
    </main>
  );
}

function MoveDialog({
  tree,
  current,
  count,
  rootLabel,
  busy,
  onCancel,
  onMove,
}: {
  tree: Folder[];
  current: string;
  count: number;
  rootLabel: string;
  busy: boolean;
  onCancel: () => void;
  onMove: (target: string) => void;
}) {
  const [filter, setFilter] = useState("");
  const [target, setTarget] = useState<string | null>(null);
  const needle = filter.trim().toLowerCase();
  const targets = tree.filter(
    (folder) => folder.path.toLowerCase() !== current.toLowerCase() && (needle === "" || folder.path.toLowerCase().includes(needle)),
  );
  return (
    <div role="dialog" aria-label="Mover itens" className="folders-dialog">
      <h2>{count === 1 ? "Mover 1 item para" : `Mover ${count} itens para`}</h2>
      <input type="search" placeholder="Filtrar pastas" value={filter} onChange={(event) => setFilter(event.target.value)} aria-label="Filtrar pastas" />
      <div className="folders-targets">
        {targets.map((folder) => (
          <label key={folder.path} data-on={target === folder.path ? "" : undefined}>
            <input type="radio" name="destino" checked={target === folder.path} onChange={() => setTarget(folder.path)} />
            {targetLabel(folder.path, rootLabel)}
          </label>
        ))}
        {targets.length === 0 ? <p className="folders-dialog-note">Nenhuma pasta com esse nome.</p> : null}
      </div>
      <p className="folders-dialog-note">Cada mídia leva junto o seu JSON do Takeout, para a data e o local não se perderem. O índice é atualizado na hora.</p>
      <div className="folders-dialog-actions">
        <button type="button" className="button-line" disabled={busy} onClick={onCancel}>
          Cancelar
        </button>
        <button type="button" className="button-solid" disabled={busy || target === null} onClick={() => target !== null && onMove(target)}>
          Mover
        </button>
      </div>
    </div>
  );
}

function NameDialog({
  title,
  action,
  initial,
  busy,
  onCancel,
  onSubmit,
}: {
  title: string;
  action: string;
  initial: string;
  busy: boolean;
  onCancel: () => void;
  onSubmit: (name: string) => void;
}) {
  const [name, setName] = useState(initial);
  const input = useRef<HTMLInputElement>(null);
  useEffect(() => {
    input.current?.select();
  }, []);
  const submit = (event: FormEvent) => {
    event.preventDefault();
    if (name.trim() !== "") {
      onSubmit(name);
    }
  };
  return (
    <form role="dialog" aria-label={title} className="folders-dialog" onSubmit={submit}>
      <h2>{title}</h2>
      <input
        ref={input}
        type="text"
        value={name}
        aria-label="Nome da pasta"
        onChange={(event) => setName(event.target.value)}
        onKeyDown={(event) => event.key === "Escape" && onCancel()}
        autoFocus
      />
      <div className="folders-dialog-actions">
        <button type="button" className="button-line" disabled={busy} onClick={onCancel}>
          Cancelar
        </button>
        <button type="submit" className="button-solid" disabled={busy || name.trim() === ""}>
          {action}
        </button>
      </div>
    </form>
  );
}
