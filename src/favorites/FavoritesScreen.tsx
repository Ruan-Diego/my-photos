import { useEffect, useMemo, useState } from "react";
import { thumbnailUrl } from "../viewer/media";
import { Viewer } from "../viewer/Viewer";
import { indexOf, openedId, viewerHash, type Entry } from "../viewer/navigation";
import { duration, tileLabel, type Item, type TimelineData } from "../timeline/timeline";
import { emptyFavorites, favoriteCount, favoritesByYear, favoritesSubtitle, FAVORITES_FILTERS, type FavoritesFilter } from "./favorites";

const HEART = "M12 20s-7-4.4-7-10a4 4 0 0 1 7-2.6A4 4 0 0 1 19 10c0 5.6-7 10-7 10z";

function useOpened(): number | null {
  const [opened, setOpened] = useState(() => openedId(window.location.hash, "favoritos"));
  useEffect(() => {
    const follow = () => setOpened(openedId(window.location.hash, "favoritos"));
    window.addEventListener("hashchange", follow);
    return () => window.removeEventListener("hashchange", follow);
  }, []);
  return opened;
}

function Tile({ item }: { item: Item }) {
  const [loaded, setLoaded] = useState(false);
  return (
    <a className="favorites-tile" href={viewerHash(item.id, "favoritos")} aria-label={tileLabel(item)} draggable={false}>
      <img
        src={thumbnailUrl(item.id)}
        alt=""
        loading="lazy"
        decoding="async"
        draggable={false}
        className={loaded ? "tile-image tile-image-loaded" : "tile-image"}
        onLoad={() => setLoaded(true)}
      />
      {item.video ? (
        <span className="tile-duration">
          <svg width="12" height="12" viewBox="0 0 24 24" aria-hidden="true">
            <path d="M7 4l13 8-13 8z" />
          </svg>
          {item.duration === null ? "Vídeo" : duration(item.duration)}
        </span>
      ) : null}
      {item.favorite ? (
        <svg className="tile-favorite" width="20" height="20" viewBox="0 0 24 24" aria-hidden="true">
          <path d={HEART} />
        </svg>
      ) : null}
    </a>
  );
}

/** F26: o que foi marcado com o coração (artboard `Favorites`), por ano. */
export function FavoritesScreen({ data, failure }: { data: TimelineData | null; failure: string | null }) {
  const opened = useOpened();
  const [filter, setFilter] = useState<FavoritesFilter>("all");
  // Desmarcado no visualizador: fica na grade (e na fila do visualizador) até ele fechar.
  const [keep, setKeep] = useState<ReadonlySet<number>>(() => new Set());
  useEffect(() => {
    setKeep((before) => {
      if (opened === null) {
        return before.size === 0 ? before : new Set();
      }
      return before.has(opened) ? before : new Set([...before, opened]);
    });
  }, [opened]);

  const years = useMemo(() => (data === null ? [] : favoritesByYear(data, filter, keep)), [data, filter, keep]);
  const entries = useMemo<Entry[]>(
    () => years.flatMap((year) => year.items.map((item, index) => ({ item, position: index + 1, dayCount: year.items.length }))),
    [years],
  );
  const openedIndex = opened === null ? -1 : indexOf(entries, opened);
  const total = data === null ? 0 : favoriteCount(data);
  const empty = data !== null && years.length === 0 ? emptyFavorites(filter, total) : null;
  const subtitle = failure !== null ? "Não foi possível ler a biblioteca" : data === null ? " " : favoritesSubtitle(total);

  return (
    <main className="page favorites" aria-labelledby="favorites-title">
      <header className="page-header">
        <div className="people-heading">
          <h1 id="favorites-title">Favoritos</h1>
          <span>{subtitle}</span>
        </div>
        <div role="group" aria-label="Mostrar" className="segmented">
          {FAVORITES_FILTERS.map((option) => (
            <button key={option.id} type="button" aria-pressed={filter === option.id} onClick={() => setFilter(option.id)}>
              {option.label}
            </button>
          ))}
        </div>
      </header>
      <div className="page-body favorites-body">
        {failure !== null ? (
          <div className="page-empty">
            <p className="page-empty-title">Não foi possível ler a biblioteca</p>
            <p className="page-empty-detail">{failure}</p>
          </div>
        ) : empty !== null ? (
          <div className="page-empty">
            <p className="page-empty-title">{empty.title}</p>
            <p className="page-empty-detail">{empty.detail}</p>
          </div>
        ) : (
          years.map((year) => (
            <section key={year.key} className="favorites-year" aria-labelledby={`favorites-${year.key}`}>
              <h2 id={`favorites-${year.key}`}>{year.title}</h2>
              <div className="favorites-grid">
                {year.items.map((item) => (
                  <Tile key={item.id} item={item} />
                ))}
              </div>
            </section>
          ))
        )}
      </div>
      {openedIndex >= 0 ? <Viewer entries={entries} index={openedIndex} base="favoritos" /> : null}
    </main>
  );
}
