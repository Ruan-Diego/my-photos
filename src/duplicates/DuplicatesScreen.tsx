import { useEffect, useMemo, useState } from "react";
import { folderHash, parentOf } from "../folders/folders";
import { invoke } from "@tauri-apps/api/core";
import { thumbnailUrl } from "../viewer/media";
import { Viewer } from "../viewer/Viewer";
import { indexOf, openedId, viewerHash, type Entry } from "../viewer/navigation";
import type { TimelineData } from "../timeline/timeline";
import {
  copyMeta,
  emptyDuplicates,
  fileName,
  filterGroups,
  folderLabel,
  groupSaving,
  groupTitle,
  KIND_LABEL,
  kindFilters,
  viewerItems,
  type Copy,
  type Duplicates,
  type Kind,
  type KindFilter,
} from "./duplicates";
import { formatCount } from "../i18n/i18n";

function useDuplicates(): [Duplicates | null, string | null] {
  const [duplicates, setDuplicates] = useState<Duplicates | null>(null);
  const [failure, setFailure] = useState<string | null>(null);
  useEffect(() => {
    invoke<Duplicates>("duplicates_list")
      .then(setDuplicates)
      .catch((error) => setFailure(String(error)));
  }, []);
  return [duplicates, failure];
}

function useOpened(): number | null {
  const [opened, setOpened] = useState(() => openedId(window.location.hash, "duplicatas"));
  useEffect(() => {
    const follow = () => setOpened(openedId(window.location.hash, "duplicatas"));
    window.addEventListener("hashchange", follow);
    return () => window.removeEventListener("hashchange", follow);
  }, []);
  return opened;
}

function CopyCard({ kind, copy }: { kind: Kind; copy: Copy }) {
  const [broken, setBroken] = useState(false);
  return (
    <div className="duplicate-copy">
      <a className="duplicate-thumb" href={viewerHash(copy.id, "duplicatas")} aria-label="Abrir no visualizador" draggable={false}>
        {broken ? null : (
          <img src={thumbnailUrl(copy.id)} alt="" loading="lazy" decoding="async" draggable={false} onError={() => setBroken(true)} />
        )}
      </a>
      <div className="duplicate-text">
        <strong>{fileName(copy.path)}</strong>
        <span>{folderLabel(copy.path)}</span>
        <span>{copyMeta(kind, copy)}</span>
        <a className="duplicate-folder" href={folderHash(parentOf(copy.path))}>
          Mostrar na pasta
        </a>
      </div>
    </div>
  );
}

/** Teto de grupos na tela; a contagem nos filtros é de todos. */
const GROUP_LIMIT = 200;

/** F29: arquivos repetidos e fotos parecidas, só para informar (artboard `Duplicates`). */
export function DuplicatesScreen({ data }: { data: TimelineData | null }) {
  const [duplicates, failure] = useDuplicates();
  const [filter, setFilter] = useState<KindFilter>("all");
  const opened = useOpened();
  const groups = useMemo(() => (duplicates === null ? [] : filterGroups(duplicates.groups, filter)), [duplicates, filter]);
  const entries = useMemo<Entry[]>(() => {
    const items = viewerItems(groups, data);
    return items.map((item, index) => ({ item, position: index + 1, dayCount: items.length }));
  }, [groups, data]);
  const openedIndex = opened === null ? -1 : indexOf(entries, opened);

  return (
    <main className="page duplicates" aria-labelledby="duplicates-title">
      <header className="page-header">
        <div className="people-heading">
          <h1 id="duplicates-title">Duplicatas</h1>
          <span>Só informativo: nada é excluído automaticamente</span>
        </div>
        {duplicates !== null ? (
          <div role="group" aria-label="Tipo de duplicata" className="segmented">
            {kindFilters(duplicates.groups).map((option) => (
              <button key={option.id} type="button" aria-pressed={filter === option.id} onClick={() => setFilter(option.id)}>
                {option.label}
              </button>
            ))}
          </div>
        ) : null}
      </header>
      <div className="page-body duplicates-body">
        {failure !== null ? (
          <div className="page-empty">
            <p className="page-empty-title">Não foi possível ler as duplicatas</p>
            <p className="page-empty-detail">{failure}</p>
          </div>
        ) : duplicates === null ? null : (
          <>
            <p className="duplicates-note">
              O Takeout costuma repetir a mesma foto na pasta do ano e na pasta de cada álbum. Esses grupos mostram onde estão as
              cópias para você decidir o que fazer em Pastas.
            </p>
            {groups.length === 0 ? (
              <div className="page-empty">
                <p className="page-empty-title">{emptyDuplicates(filter, duplicates.hashed, duplicates.photos).title}</p>
                <p className="page-empty-detail">{emptyDuplicates(filter, duplicates.hashed, duplicates.photos).detail}</p>
              </div>
            ) : (
              groups.slice(0, GROUP_LIMIT).map((group) => (
                <section key={`${group.kind}-${group.copies[0].id}`} className="duplicate-group" aria-label={groupTitle(group)}>
                  <div className="duplicate-head">
                    <span className={`duplicate-badge duplicate-badge-${group.kind}`}>{KIND_LABEL[group.kind]}</span>
                    <strong>{groupTitle(group)}</strong>
                    <span className="duplicate-saving">{groupSaving(group)}</span>
                  </div>
                  <div className="duplicate-copies">
                    {group.copies.map((copy) => (
                      <CopyCard key={copy.id} kind={group.kind} copy={copy} />
                    ))}
                  </div>
                </section>
              ))
            )}
            {groups.length > GROUP_LIMIT ? (
              <p className="problems-note">
                Mostrando {GROUP_LIMIT} de {formatCount(groups.length)} grupos, os com mais espaço repetido primeiro.
              </p>
            ) : null}
          </>
        )}
      </div>
      {openedIndex >= 0 ? <Viewer entries={entries} index={openedIndex} base="duplicatas" /> : null}
    </main>
  );
}
