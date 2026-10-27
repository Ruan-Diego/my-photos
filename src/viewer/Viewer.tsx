import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { fit, icon, originalUrl, thumbnailUrl, useMediaInfo } from "./media";
import { DetailsPanel, useDetailsOpen } from "./DetailsPanel";
import { useToggleFavorite } from "../favorites/FavoriteContext";
import { VideoPlayer } from "./VideoPlayer";
import {
  dateLine,
  fallbackNote,
  positionLine,
  stripWindow,
  backHash,
  backLabel,
  viewerHash,
  type ViewerBase,
  type Entry,
  type Zoom,
} from "./navigation";

/** Foto no visualizador (F19); vídeo no player (F20, artboard `Video`). */
export function Viewer({ entries, index, base = "fotos" }: { entries: Entry[]; index: number; base?: ViewerBase }) {
  const item = entries[index].item;
  return item.video ? <VideoPlayer key={item.id} item={item} base={base} /> : <PhotoViewer entries={entries} index={index} base={base} />;
}

function PhotoViewer({ entries, index, base }: { entries: Entry[]; index: number; base: ViewerBase }) {
  const entry = entries[index];
  const item = entry.item;
  const info = useMediaInfo(item.id);
  const toggleFavorite = useToggleFavorite();
  const [zoom, setZoom] = useState<Zoom>("fit");
  const [originalReady, setOriginalReady] = useState(false);
  const [detailsOpen, toggleDetails] = useDetailsOpen();
  const stage = useRef<HTMLDivElement>(null);
  const [area, setArea] = useState({ width: 0, height: 0 });
  const drag = useRef<{ x: number; y: number; left: number; top: number } | null>(null);

  const viewable = info?.viewable === true && info.id === item.id;
  const note = info !== null && info.id === item.id && !info.viewable ? fallbackNote(info) : null;
  const previous = index > 0 ? entries[index - 1].item.id : null;
  const next = index < entries.length - 1 ? entries[index + 1].item.id : null;

  useEffect(() => {
    setZoom("fit");
    setOriginalReady(false);
  }, [item.id]);

  useLayoutEffect(() => {
    const element = stage.current;
    if (element === null) {
      return;
    }
    const measure = () => {
      const styles = window.getComputedStyle(element);
      setArea({
        width: element.clientWidth - parseFloat(styles.paddingLeft) - parseFloat(styles.paddingRight),
        height: element.clientHeight - parseFloat(styles.paddingTop) - parseFloat(styles.paddingBottom),
      });
    };
    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(element);
    return () => observer.disconnect();
  }, []);

  // Setas andam, Esc volta para a grade.
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "ArrowLeft" && previous !== null) {
        window.location.hash = viewerHash(previous, base);
      } else if (event.key === "ArrowRight" && next !== null) {
        window.location.hash = viewerHash(next, base);
      } else if (event.key === "Escape") {
        window.location.hash = backHash(base);
      } else {
        return;
      }
      event.preventDefault();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [previous, next, base]);

  // A próxima já vem do disco enquanto esta é vista.
  useEffect(() => {
    if (next === null || !viewable) {
      return;
    }
    const timer = window.setTimeout(() => {
      new Image().src = originalUrl(next);
    }, 400);
    return () => window.clearTimeout(timer);
  }, [next, viewable]);

  // A proporção da miniatura já vem girada; a do índice nem sempre.
  const box = fit(item.ratio > 0 ? item.ratio : 1, area.width, area.height);
  const actual = zoom === "actual" && viewable;
  const [stripStart, stripEnd] = stripWindow(index, entries.length);

  return (
    <div className="viewer" role="dialog" aria-modal="true" aria-label="Visualizar foto">
      <div className="viewer-main">
        <header className="viewer-header">
          <a className="viewer-icon" href={backHash(base)} aria-label={backLabel(base)}>
            <svg width="22" height="22" viewBox="0 0 24 24" {...icon} aria-hidden="true">
              <path d="M19 12H5M11 6l-6 6 6 6" />
            </svg>
          </a>
          <div className="viewer-title">
            <span>{dateLine(item)}</span>
            <span>{positionLine(entry)}</span>
          </div>
          <div role="group" aria-label="Zoom" className="viewer-zoom">
            <button type="button" aria-pressed={!actual} onClick={() => setZoom("fit")}>
              Ajustar
            </button>
            <button type="button" aria-pressed={actual} disabled={!viewable} onClick={() => setZoom("actual")}>
              100%
            </button>
          </div>
          <button
            type="button"
            className="viewer-icon viewer-favorite"
            aria-label={item.favorite ? "Remover dos favoritos" : "Favoritar"}
            title={item.favorite ? "Remover dos favoritos" : "Favoritar"}
            aria-pressed={item.favorite}
            disabled={toggleFavorite === null}
            onClick={() => toggleFavorite?.(item.id, !item.favorite)}
          >
            <svg width="22" height="22" viewBox="0 0 24 24" {...icon} fill={item.favorite ? "currentColor" : "none"} aria-hidden="true">
              <path d="M12 20s-7-4.4-7-10a4 4 0 0 1 7-2.6A4 4 0 0 1 19 10c0 5.6-7 10-7 10z" />
            </svg>
          </button>
          <button
            type="button"
            className="viewer-icon viewer-toggle"
            aria-label={detailsOpen ? "Ocultar detalhes" : "Mostrar detalhes"}
            aria-pressed={detailsOpen}
            onClick={toggleDetails}
          >
            <svg width="22" height="22" viewBox="0 0 24 24" {...icon} aria-hidden="true">
              <circle cx="12" cy="12" r="9" />
              <path d="M12 11v6M12 7.5v.01" />
            </svg>
          </button>
          <button type="button" className="viewer-icon" aria-label="Mais ações" disabled title="Ações chegam com as operações de arquivo">
            <svg width="22" height="22" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true">
              <circle cx="5" cy="12" r="1.7" />
              <circle cx="12" cy="12" r="1.7" />
              <circle cx="19" cy="12" r="1.7" />
            </svg>
          </button>
        </header>

        <div
          ref={stage}
          className={actual ? "viewer-stage viewer-stage-actual" : "viewer-stage"}
          onPointerDown={(event) => {
            if (!actual) {
              return;
            }
            const element = event.currentTarget;
            element.setPointerCapture(event.pointerId);
            drag.current = { x: event.clientX, y: event.clientY, left: element.scrollLeft, top: element.scrollTop };
          }}
          onPointerMove={(event) => {
            const start = drag.current;
            if (start === null) {
              return;
            }
            event.currentTarget.scrollLeft = start.left - (event.clientX - start.x);
            event.currentTarget.scrollTop = start.top - (event.clientY - start.y);
          }}
          onPointerUp={() => {
            drag.current = null;
          }}
        >
          {actual ? (
            <img className="viewer-actual" src={originalUrl(item.id)} alt={info?.filename ?? ""} draggable={false} />
          ) : (
            <div className="viewer-photo" role="img" aria-label={info === null ? "Foto" : `Foto: ${info.filename}`} style={{ width: box.width, height: box.height }}>
              <img className="viewer-layer" src={thumbnailUrl(item.id)} alt="" draggable={false} />
              {viewable ? (
                <img
                  key={item.id}
                  className={originalReady ? "viewer-layer viewer-layer-ready" : "viewer-layer viewer-layer-loading"}
                  src={originalUrl(item.id)}
                  alt=""
                  draggable={false}
                  onLoad={() => setOriginalReady(true)}
                />
              ) : null}
              {note !== null ? <span className="viewer-note">{note}</span> : null}
            </div>
          )}
          {previous !== null ? (
            <a className="viewer-arrow viewer-arrow-previous" href={viewerHash(previous, base)} aria-label="Foto anterior">
              <svg width="22" height="22" viewBox="0 0 24 24" {...icon} strokeWidth={2} aria-hidden="true">
                <path d="M15 6l-6 6 6 6" />
              </svg>
            </a>
          ) : null}
          {next !== null ? (
            <a className="viewer-arrow viewer-arrow-next" href={viewerHash(next, base)} aria-label="Próxima">
              <svg width="22" height="22" viewBox="0 0 24 24" {...icon} strokeWidth={2} aria-hidden="true">
                <path d="M9 6l6 6-6 6" />
              </svg>
            </a>
          ) : null}
        </div>

        <nav className="viewer-strip" aria-label="Fotos vizinhas">
          {entries.slice(stripStart, stripEnd).map((neighbor) => (
            <a
              key={neighbor.item.id}
              href={viewerHash(neighbor.item.id, base)}
              className="viewer-strip-tile"
              aria-current={neighbor.item.id === item.id ? "true" : undefined}
              aria-label={dateLine(neighbor.item)}
            >
              <img src={thumbnailUrl(neighbor.item.id)} alt="" draggable={false} />
            </a>
          ))}
        </nav>
      </div>
      {detailsOpen ? <DetailsPanel item={item} info={info} /> : null}
    </div>
  );
}
