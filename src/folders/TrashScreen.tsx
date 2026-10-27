import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { thumbnailUrl } from "../viewer/media";
import { itemsLabel, outcomeText, purgeText, trashName, trashOrigin, type Outcome, type TrashItem } from "./folders";

function errorText(error: unknown): string {
  return typeof error === "string" ? error : error instanceof Error ? error.message : String(error);
}

/** Lixeira do app em `E:\.myphotos\trash` (AD-009, artboard `Trash`). */
export function TrashScreen({ rootLabel, writable, onChanged }: { rootLabel: string; writable: boolean; onChanged: () => void }) {
  const [items, setItems] = useState<TrashItem[] | null>(null);
  const [failure, setFailure] = useState<string | null>(null);
  const [notice, setNotice] = useState<{ text: string; warn: boolean } | null>(null);
  const [busy, setBusy] = useState(false);
  const [version, setVersion] = useState(0);
  const now = Date.now();

  useEffect(() => {
    invoke<TrashItem[]>("trash_list")
      .then((value) => {
        setItems(value);
        setFailure(null);
      })
      .catch((error) => setFailure(errorText(error)));
  }, [version]);

  const run = <T,>(work: Promise<T>, done: (value: T) => { text: string; warn: boolean }) => {
    setBusy(true);
    work
      .then((value) => {
        setNotice(done(value));
        onChanged();
      })
      .catch((error) => setNotice({ text: errorText(error), warn: true }))
      .finally(() => {
        setBusy(false);
        setVersion((value) => value + 1);
      });
  };

  const trashPath = `${rootLabel.replace(/\\$/, "")}\\.myphotos\\trash`;

  return (
    <main className="page trash" aria-labelledby="page-title">
      <header className="page-header trash-header">
        <div className="trash-title">
          <h1 id="page-title">Lixeira</h1>
          <span>{items === null ? " " : `${itemsLabel(items.length)} · cada item fica 30 dias antes de ser apagado`}</span>
        </div>
        <button
          type="button"
          className="button-solid trash-restore-all"
          disabled={!writable || busy || items === null || items.length === 0}
          onClick={() =>
            run(invoke<Outcome>("trash_restore_all"), (outcome) => ({ text: outcomeText(outcome, "restaurado"), warn: outcome.skipped.length > 0 }))
          }
        >
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
            <path d="M4 12a8 8 0 1 0 2.3-5.7M4 4v5h5" />
          </svg>
          Restaurar tudo
        </button>
      </header>

      <div className="page-body">
        <div className="trash-body">
          <p className="trash-info">
            <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" aria-hidden="true">
              <circle cx="12" cy="12" r="9" />
              <path d="M12 11v6M12 7.5v.01" />
            </svg>
            <span>
              A lixeira mora no próprio HD, em <code>{trashPath}</code>, porque a Lixeira do Windows nem sempre existe em discos USB. Restaurar devolve a mídia e o JSON do Takeout
              para a pasta de origem.
            </span>
          </p>

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
              <p className="page-empty-title">Não foi possível ler a lixeira</p>
              <p className="page-empty-detail">{failure}</p>
            </div>
          ) : items !== null && items.length === 0 ? (
            <div className="trash-empty">
              <strong>A lixeira está vazia</strong>
              <span>O que for excluído em Pastas fica aqui por 30 dias.</span>
            </div>
          ) : (
            <ul className="trash-list">
              {(items ?? []).map((item) => {
                const left = purgeText(item.purgeAfter, now);
                return (
                  <li key={item.id}>
                    <span className="trash-thumb" aria-hidden="true">
                      {item.mediaId !== null ? (
                        <img src={thumbnailUrl(item.mediaId)} alt="" loading="lazy" />
                      ) : (
                        <svg width="28" height="28" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round">
                          <path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" />
                        </svg>
                      )}
                    </span>
                    <div className="trash-what">
                      <strong>{trashName(item)}</strong>
                      <span>{trashOrigin(item, rootLabel)}</span>
                    </div>
                    <span className={left.warn ? "trash-left trash-left-warn" : "trash-left"}>{left.text}</span>
                    <button
                      type="button"
                      className="button-line"
                      disabled={!writable || busy}
                      onClick={() => run(invoke("trash_restore", { id: item.id }), () => ({ text: `${trashName(item)} voltou para a pasta de origem.`, warn: false }))}
                    >
                      Restaurar
                    </button>
                  </li>
                );
              })}
            </ul>
          )}
        </div>
      </div>
    </main>
  );
}
