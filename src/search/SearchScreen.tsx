import { useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { duration, tileLabel, FAVORITE, VIDEO, type Item, type TimelineData } from "../timeline/timeline";
import { thumbnailUrl } from "../viewer/media";
import { indexOf, openedId, TIMELINE_HASH, viewerHash, type Entry } from "../viewer/navigation";
import { Viewer } from "../viewer/Viewer";
import { personFromHash } from "../people/people";
import { placeFromHash } from "../locations/locations";
import { albumFromHash } from "../albums/albums";
import { t } from "../i18n/i18n";
import {
  chips,
  folderLabel,
  invalidDate,
  isBlank,
  NO_FILTERS,
  noType,
  removeChip,
  resultsTitle,
  toQuery,
  type Filters,
  type SearchIds,
  type SearchOptions,
} from "./query";

/** Tiles postos de cada vez; mais entram ao rolar. */
const PAGE = 240;
const DEBOUNCE_MS = 200;

/** id → item, com o que a linha do tempo já trouxe (proporção, vídeo, data). */
function itemsById(data: TimelineData | null): Map<number, Item> {
  const map = new Map<number, Item>();
  if (data === null) {
    return map;
  }
  data.ids.forEach((id, index) => {
    map.set(id, {
      id,
      ratio: data.ratios[index],
      video: (data.flags[index] & VIDEO) !== 0,
      favorite: (data.flags[index] & FAVORITE) !== 0,
      duration: data.durations[index],
      time: data.times[index],
      offset: data.offsets[index],
    });
  });
  return map;
}

/** Os filtros com que a tela abre: só a pessoa (de Pessoas), o lugar (de Locais) ou o álbum (de Álbuns). */
function fromHash(hash: string): Filters {
  const person = personFromHash(hash);
  if (person !== null) {
    return { ...NO_FILTERS, people: [person] };
  }
  const album = albumFromHash(hash);
  if (album !== null) {
    return { ...NO_FILTERS, album };
  }
  const place = placeFromHash(hash);
  return place === null ? NO_FILTERS : { ...NO_FILTERS, place };
}

function useOpenedResult(): number | null {
  const [opened, setOpened] = useState(() => openedId(window.location.hash, "buscar"));
  useEffect(() => {
    const follow = () => setOpened(openedId(window.location.hash, "buscar"));
    window.addEventListener("hashchange", follow);
    return () => window.removeEventListener("hashchange", follow);
  }, []);
  return opened;
}

function useSearchOptions(): SearchOptions {
  const [options, setOptions] = useState<SearchOptions>({ people: [], folders: [], albums: [] });
  useEffect(() => {
    invoke<SearchOptions>("search_filters")
      .then(setOptions)
      .catch(() => undefined);
  }, []);
  return options;
}

function Tile({ item }: { item: Item }) {
  return (
    <a className="search-tile" href={viewerHash(item.id, "buscar")} aria-label={tileLabel(item)} draggable={false}>
      <img src={thumbnailUrl(item.id)} alt="" loading="lazy" decoding="async" draggable={false} />
      {item.video ? (
        <span className="tile-duration">
          <svg width="12" height="12" viewBox="0 0 24 24" aria-hidden="true">
            <path d="M7 4l13 8-13 8z" />
          </svg>
          {item.duration === null ? t("timeline.video") : duration(item.duration)}
        </span>
      ) : null}
    </a>
  );
}

/** F22: tela de busca (artboard `Search`), aberta pela caixa da timeline ou por Ctrl F. */
export function SearchScreen({ data }: { data: TimelineData | null }) {
  const opened = useOpenedResult();
  const options = useSearchOptions();
  const [text, setText] = useState("");
  const [filters, setFilters] = useState<Filters>(() => fromHash(window.location.hash));
  const [found, setFound] = useState<SearchIds | null>(null);
  const [failure, setFailure] = useState<string | null>(null);
  const [shown, setShown] = useState(PAGE);
  const input = useRef<HTMLInputElement>(null);
  const sentinel = useRef<HTMLDivElement>(null);
  const request = useRef(0);

  const blank = isBlank(text, filters);
  const query = useMemo(() => toQuery(text, filters), [text, filters]);

  // Uma pessoa (F23), um lugar (F24) ou um álbum (F25) chega como `#/buscar?pessoa=7`, `?local=` ou `?album=`.
  useEffect(() => {
    const follow = () => {
      const hash = window.location.hash;
      if (personFromHash(hash) !== null || placeFromHash(hash) !== null || albumFromHash(hash) !== null) {
        setText("");
        setFilters(fromHash(hash));
      }
    };
    window.addEventListener("hashchange", follow);
    return () => window.removeEventListener("hashchange", follow);
  }, []);

  useEffect(() => {
    if (opened === null) {
      input.current?.focus();
    }
  }, [opened]);

  // Cada mudança pergunta de novo, um pouco depois da última tecla.
  useEffect(() => {
    const current = ++request.current;
    if (blank || noType(filters)) {
      setFound(blank ? null : { ids: [], photos: 0, videos: 0 });
      return;
    }
    const timer = window.setTimeout(() => {
      invoke<SearchIds>("search_media", { query })
        .then((result) => {
          if (current === request.current) {
            setFound(result);
            setFailure(null);
            setShown(PAGE);
          }
        })
        .catch((error) => {
          if (current === request.current) {
            setFailure(String(error));
          }
        });
    }, DEBOUNCE_MS);
    return () => window.clearTimeout(timer);
  }, [query, blank, filters]);

  const byId = useMemo(() => itemsById(data), [data]);
  const results = useMemo(
    () => (found === null ? [] : found.ids.flatMap((id) => byId.get(id) ?? [])),
    [found, byId],
  );
  const entries = useMemo<Entry[]>(
    () => results.map((item, index) => ({ item, position: index + 1, dayCount: results.length })),
    [results],
  );
  const openedIndex = opened === null ? -1 : indexOf(entries, opened);

  // Mais resultados quando o fim da grade chega perto da tela.
  useEffect(() => {
    const element = sentinel.current;
    if (element === null || shown >= results.length) {
      return;
    }
    const observer = new IntersectionObserver(
      (seen) => {
        if (seen.some((entry) => entry.isIntersecting)) {
          setShown((count) => count + PAGE);
        }
      },
      { rootMargin: "600px" },
    );
    observer.observe(element);
    return () => observer.disconnect();
  }, [shown, results.length]);

  const active = chips(filters, options.people, options.albums);
  const set = (change: Partial<Filters>) => setFilters((current) => ({ ...current, ...change }));
  const togglePerson = (id: number) =>
    set({ people: filters.people.includes(id) ? filters.people.filter((person) => person !== id) : [...filters.people, id] });

  return (
    <main className="page search" aria-labelledby="search-title">
      <header className="search-header">
        <label className="search-box">
          <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" aria-hidden="true">
            <path d="M11 18a7 7 0 1 0 0-14 7 7 0 0 0 0 14zM20 20l-4-4" />
          </svg>
          <span className="visually-hidden">{t("section.search")}</span>
          <input
            ref={input}
            type="search"
            value={text}
            placeholder={t("search.placeholder")}
            onChange={(event) => setText(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === "Escape") {
                event.preventDefault();
                if (text !== "") {
                  setText("");
                } else {
                  window.location.hash = TIMELINE_HASH;
                }
              }
            }}
          />
          <kbd>Esc</kbd>
        </label>
        {active.length > 0 ? (
          <div className="search-chips">
            {active.map((chip) => (
              <button key={chip.key} type="button" className="search-chip" aria-label={t("search.removeFilter", { label: chip.label })} onClick={() => setFilters(removeChip(filters, chip.key))}>
                {chip.label}
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.4" strokeLinecap="round" aria-hidden="true">
                  <path d="M6 6l12 12M18 6L6 18" />
                </svg>
              </button>
            ))}
            <button type="button" className="search-clear" onClick={() => setFilters(NO_FILTERS)}>
              {t("search.clearFilters")}
            </button>
          </div>
        ) : null}
      </header>

      <div className="search-body">
        <aside className="search-filters" aria-label={t("search.filters")}>
          <fieldset>
            <legend>{t("search.date")}</legend>
            <div className="search-dates">
              {(["from", "until"] as const).map((key) => (
                <label key={key}>
                  {key === "from" ? t("search.from") : t("search.until")}
                  <input
                    type="text"
                    inputMode="numeric"
                    placeholder="dd/mm/aaaa"
                    value={filters[key]}
                    aria-invalid={invalidDate(filters[key])}
                    onChange={(event) => set({ [key]: event.target.value })}
                  />
                </label>
              ))}
            </div>
          </fieldset>

          <fieldset>
            <legend>{t("search.type")}</legend>
            <label className="search-check">
              <input type="checkbox" checked={filters.photos} onChange={(event) => set({ photos: event.target.checked })} /> {t("section.timeline")}
              {found !== null && !blank ? <span>{found.photos}</span> : null}
            </label>
            <label className="search-check">
              <input type="checkbox" checked={filters.videos} onChange={(event) => set({ videos: event.target.checked })} /> {t("indexing.countVideos")}
              {found !== null && !blank ? <span>{found.videos}</span> : null}
            </label>
            <label className="search-check">
              <input type="checkbox" checked={filters.favoritesOnly} onChange={(event) => set({ favoritesOnly: event.target.checked })} /> {t("search.onlyFavorites")}
            </label>
          </fieldset>

          <fieldset>
            <legend>{t("section.people")}</legend>
            {options.people.length > 0 ? (
              <div className="search-people">
                {options.people.map((person) => (
                  <button key={person.id} type="button" className="search-person" aria-pressed={filters.people.includes(person.id)} onClick={() => togglePerson(person.id)}>
                    <span aria-hidden="true">{person.name.slice(0, 1).toUpperCase()}</span>
                    {person.name}
                  </button>
                ))}
              </div>
            ) : (
              <p className="search-muted">{t("details.noPeople")}</p>
            )}
          </fieldset>

          <label className="search-field">
            {t("details.place")}
            <input type="text" placeholder={t("search.placeName")} value={filters.place} onChange={(event) => set({ place: event.target.value })} />
          </label>

          <label className="search-field">
            {t("details.folder")}
            <select value={filters.folder ?? ""} onChange={(event) => set({ folder: event.target.value === "" ? null : event.target.value })}>
              <option value="">{t("search.allFolders")}</option>
              {options.folders.map((folder) => (
                <option key={folder} value={folder}>
                  {folderLabel(folder)}
                </option>
              ))}
            </select>
          </label>
        </aside>

        <section className="search-results" aria-label={t("search.resultsLabel")}>
          {blank ? (
            <div className="search-heading search-heading-blank">
              <h1 id="search-title">{t("section.search")}</h1>
              <span>{t("search.hint")}</span>
            </div>
          ) : failure !== null ? (
            <div className="search-heading">
              <h1 id="search-title">{t("search.failed")}</h1>
              <span>{failure}</span>
            </div>
          ) : (
            <>
              <div className="search-heading" aria-live="polite">
                <h1 id="search-title">{found === null ? t("search.searching") : resultsTitle(results.length)}</h1>
                {results.length > 0 ? <span>{t("search.sortedByDate")}</span> : null}
              </div>
              <div className="search-grid">
                {results.slice(0, shown).map((item) => (
                  <Tile key={item.id} item={item} />
                ))}
              </div>
              <div ref={sentinel} aria-hidden="true" />
            </>
          )}
        </section>
      </div>
      {openedIndex >= 0 ? <Viewer entries={entries} index={openedIndex} base="buscar" /> : null}
    </main>
  );
}
