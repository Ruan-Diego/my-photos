import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { MotionConfig } from "motion/react";
import { linkedMotionLibraries } from "./motion/libraries";
import { failedCard, failureText, needsOnboarding, type IndexingFinished, type LibraryFolder } from "./onboarding/flow";
import { IndexingScreen, TILES } from "./indexing/IndexingScreen";
import {
  FINISHED_EVENT,
  indexingCard,
  interrupted,
  interruptedCard,
  missingThumbnails,
  thumbnailsCard,
  logLine,
  PAUSED,
  PROGRESS_EVENT,
  pushLines,
  starting as startingProgress,
  thumbnailLine,
  type LogLine,
  type Progress,
  type RecentThumbnail,
} from "./indexing/progress";
import { Onboarding } from "./onboarding/Onboarding";
import { Appearance } from "./shell/Appearance";
import { diskCard, type LibrarySummary } from "./shell/library";
import { SectionPage } from "./shell/SectionPage";
import { sectionById, sectionFromHash, type SectionId } from "./shell/sections";
import { AlbumsScreen } from "./albums/AlbumsScreen";
import { LocationsScreen } from "./locations/LocationsScreen";
import { PeopleScreen } from "./people/PeopleScreen";
import { SearchScreen } from "./search/SearchScreen";
import { SEARCH_HASH } from "./search/query";
import { Sidebar } from "./shell/Sidebar";
import { TimelineScreen } from "./timeline/TimelineScreen";
import { FavoritesScreen } from "./favorites/FavoritesScreen";
import { FavoriteContext, type ToggleFavorite } from "./favorites/FavoriteContext";
import { withFavorite } from "./favorites/favorites";
import type { TimelineData } from "./timeline/timeline";
import { applyTheme, loadTheme, saveTheme, type ThemeChoice } from "./theme/theme";
import "@fontsource/hanken-grotesk/400.css";
import "@fontsource/hanken-grotesk/500.css";
import "@fontsource/hanken-grotesk/600.css";
import "@fontsource/hanken-grotesk/700.css";
import "./theme/tokens.css";
import "./App.css";

const FADE_MS = 200;

function errorText(error: unknown): string {
  return typeof error === "string" ? error : error instanceof Error ? error.message : String(error);
}

function useSection(): SectionId {
  const [section, setSection] = useState(() => sectionFromHash(window.location.hash));
  useEffect(() => {
    const follow = () => setSection(sectionFromHash(window.location.hash));
    window.addEventListener("hashchange", follow);
    return () => window.removeEventListener("hashchange", follow);
  }, []);
  return section;
}

function useTheme(): [ThemeChoice, (choice: ThemeChoice) => void] {
  // index.html já aplicou a escolha antes da primeira pintura.
  const [theme, setTheme] = useState(loadTheme);
  const choose = useCallback((choice: ThemeChoice) => {
    const root = document.documentElement;
    const still = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    if (!still) {
      // Só cor e fundo, 200 ms; com reduzir movimento a troca é seca.
      root.classList.add("theme-fade");
      window.setTimeout(() => root.classList.remove("theme-fade"), FADE_MS + 50);
    }
    applyTheme(choice, root);
    saveTheme(choice);
    setTheme(choice);
  }, []);
  return [theme, choose];
}

/** Resumo do disco; `undefined` enquanto a primeira resposta não chega. */
function useLibrary(): [LibrarySummary | null | undefined, () => void] {
  const [summary, setSummary] = useState<LibrarySummary | null | undefined>(undefined);
  const reload = useCallback(() => {
    invoke<LibrarySummary>("library_summary")
      .then(setSummary)
      .catch(() => setSummary(null));
  }, []);
  useEffect(reload, [reload]);
  return [summary, reload];
}

/** F18: as mídias da linha do tempo, relidas quando o resumo do disco muda (fim da indexação). */
function useTimeline(
  enabled: boolean,
  summary: LibrarySummary | null | undefined,
): [TimelineData | null, string | null, ToggleFavorite] {
  const [data, setData] = useState<TimelineData | null>(null);
  const [failure, setFailure] = useState<string | null>(null);
  useEffect(() => {
    if (!enabled || summary === undefined) {
      return;
    }
    let current = true;
    invoke<TimelineData>("timeline")
      .then((timeline) => {
        if (current) {
          setData(timeline);
          setFailure(null);
        }
      })
      .catch((error) => {
        if (current) {
          setFailure(errorText(error));
        }
      });
    return () => {
      current = false;
    };
  }, [enabled, summary]);
  // F26: o coração muda na hora e volta atrás se o banco recusar.
  const toggleFavorite = useCallback<ToggleFavorite>((id, favorite) => {
    setData((before) => (before === null ? before : withFavorite(before, id, favorite)));
    invoke<boolean>("favorite_set", { id, favorite }).catch(() =>
      setData((before) => (before === null ? before : withFavorite(before, id, !favorite))),
    );
  }, []);
  return [data, failure, toggleFavorite];
}

const THUMBNAIL_POLL_MS = 1500;

function App() {
  const active = useSection();
  const [theme, setTheme] = useTheme();
  const [summary, reloadSummary] = useLibrary();
  const [folder, setFolder] = useState<LibraryFolder | null>(null);
  const [progress, setProgress] = useState<Progress | null>(null);
  // Código do Rust quando a última indexação falhou; texto quando nem deu para começar.
  const [indexError, setIndexError] = useState<string | null>(null);
  const [startError, setStartError] = useState<string | null>(null);
  const [starting, setStarting] = useState(false);
  // F17: a tela de progresso (artboard `Indexing`) ou o app com o cartão na lateral.
  const [showProgress, setShowProgress] = useState(false);
  const [paused, setPaused] = useState(false);
  const [pausing, setPausing] = useState(false);
  const [runStartedAt, setRunStartedAt] = useState(0);
  const [now, setNow] = useState(() => Date.now());
  const [thumbnails, setThumbnails] = useState<RecentThumbnail[]>([]);
  const [lines, setLines] = useState<LogLine[]>([]);
  const seenThumbnails = useRef(new Set<number>());
  // A busca (F22) monta os resultados com os itens da linha do tempo.
  const [timeline, timelineFailure, toggleFavorite] = useTimeline(
    active === "timeline" || active === "search" || active === "favorites",
    summary,
  );

  useEffect(() => {
    linkedMotionLibraries();
  }, []);

  // Ctrl F abre a busca (F22) de qualquer tela; nela, só volta o foco à caixa.
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if ((event.ctrlKey || event.metaKey) && !event.altKey && event.key.toLowerCase() === "f") {
        event.preventDefault();
        if (!window.location.hash.startsWith(SEARCH_HASH)) {
          window.location.hash = SEARCH_HASH;
        } else {
          document.querySelector<HTMLInputElement>(".search-box input")?.focus();
        }
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  // A indexação roda no Rust; daqui só se ouve o progresso e o fim.
  useEffect(() => {
    const progressOff = listen<Progress>(PROGRESS_EVENT, (event) => {
      setProgress(event.payload);
      if (event.payload.recent.length > 0) {
        setLines((current) => pushLines(current, event.payload.recent.map(logLine)));
      }
    });
    const finishedOff = listen<IndexingFinished>(FINISHED_EVENT, (event) => {
      setPausing(false);
      if (event.payload.error === PAUSED) {
        setPaused(true);
      } else {
        setProgress(null);
        setPaused(false);
        setShowProgress(false);
        setIndexError(event.payload.error);
      }
      reloadSummary();
    });
    return () => {
      void progressOff.then((off) => off());
      void finishedOff.then((off) => off());
    };
  }, [reloadSummary]);

  const running = progress !== null && !paused;

  // Relógio do "cerca de N min restantes".
  useEffect(() => {
    if (!running) {
      return;
    }
    const timer = window.setInterval(() => setNow(Date.now()), 1000);
    return () => window.clearInterval(timer);
  }, [running]);

  // "Chegando agora": as miniaturas mais novas, enquanto a tela está aberta.
  const watchThumbnails = showProgress && progress !== null;
  const makingThumbnails = running && progress?.stage === "thumbnails";
  useEffect(() => {
    if (!watchThumbnails) {
      return;
    }
    const fetchThumbnails = () => {
      invoke<RecentThumbnail[]>("recent_thumbnails", { limit: TILES })
        .then((recent) => {
          setThumbnails(recent);
          const fresh = recent.filter((thumbnail) => !seenThumbnails.current.has(thumbnail.mediaId)).reverse();
          fresh.forEach((thumbnail) => seenThumbnails.current.add(thumbnail.mediaId));
          if (makingThumbnails && fresh.length > 0) {
            setLines((current) => pushLines(current, fresh.map(thumbnailLine)));
          }
        })
        .catch(() => undefined);
    };
    fetchThumbnails();
    if (!makingThumbnails) {
      return;
    }
    const timer = window.setInterval(fetchThumbnails, THUMBNAIL_POLL_MS);
    return () => window.clearInterval(timer);
  }, [watchThumbnails, makingThumbnails]);

  const onboarding = summary !== undefined && needsOnboarding(summary) && progress === null && !starting;

  useEffect(() => {
    if (!onboarding || folder !== null) {
      return;
    }
    invoke<LibraryFolder>("library_folder")
      .then(setFolder)
      .catch((error) => setStartError(errorText(error)));
  }, [onboarding, folder]);

  const startIndexing = useCallback(() => {
    setStarting(true);
    setStartError(null);
    setIndexError(null);
    invoke("start_indexing")
      .then(() => {
        setPaused(false);
        setProgress((current) => current ?? startingProgress());
        setLines([]);
        setRunStartedAt(Date.now());
        setNow(Date.now());
        setShowProgress(true);
      })
      .catch((error) => setStartError(errorText(error)))
      .finally(() => setStarting(false));
  }, []);

  const pauseIndexing = useCallback(() => {
    setPausing(true);
    invoke("pause_indexing").catch(() => setPausing(false));
  }, []);

  if (summary === undefined) {
    // Fundo já pintado pelo index.html; nada a mostrar por um instante.
    return null;
  }

  if (onboarding) {
    return (
      <MotionConfig reducedMotion="user">
        <Onboarding
          folder={folder}
          starting={starting}
          failure={startError ?? (indexError === null ? null : `A indexação parou: ${failureText(indexError)}.`)}
          onStart={startIndexing}
        />
      </MotionConfig>
    );
  }

  const root = summary?.rootPath ?? folder?.rootPath ?? null;

  if (showProgress && progress !== null) {
    return (
      <MotionConfig reducedMotion="user">
        <IndexingScreen
          root={root}
          progress={progress}
          paused={paused}
          pausing={pausing || starting}
          elapsedMs={now - runStartedAt}
          thumbnails={thumbnails}
          lines={lines}
          onPause={pauseIndexing}
          onResume={startIndexing}
          onBrowse={() => setShowProgress(false)}
        />
      </MotionConfig>
    );
  }

  let disk = diskCard(summary);
  let onDiskAction: (() => void) | undefined;
  if (progress !== null) {
    disk = indexingCard(root, progress, paused);
    onDiskAction = paused ? startIndexing : () => setShowProgress(true);
  } else if (indexError !== null) {
    disk = failedCard(indexError);
    onDiskAction = startIndexing;
  } else if (startError !== null) {
    disk = { tone: "warn", title: "A indexação não começou", detail: startError, action: null };
  } else if (interrupted(summary)) {
    disk = interruptedCard(summary?.mediaCount ?? 0);
    onDiskAction = startIndexing;
  } else if (missingThumbnails(summary ?? null) > 0) {
    disk = thumbnailsCard(missingThumbnails(summary ?? null));
    onDiskAction = startIndexing;
  } else if (summary?.connected === true) {
    // "Procurar arquivos novos": varredura incremental.
    onDiskAction = startIndexing;
  }
  const section = sectionById(active);
  return (
    <MotionConfig reducedMotion="user">
      <FavoriteContext.Provider value={summary?.connected === true ? toggleFavorite : null}>
      <div className="shell">
        <Sidebar active={active} disk={disk} onDiskAction={onDiskAction} />
        {section.id === "timeline" ? (
          <TimelineScreen data={timeline} failure={timelineFailure} />
        ) : section.id === "search" ? (
          <SearchScreen data={timeline} />
        ) : section.id === "people" ? (
          <PeopleScreen />
        ) : section.id === "locations" ? (
          <LocationsScreen />
        ) : section.id === "albums" ? (
          <AlbumsScreen />
        ) : section.id === "favorites" ? (
          <FavoritesScreen data={timeline} failure={timelineFailure} />
        ) : (
          <SectionPage key={section.id} section={section}>
            {section.id === "settings" ? (
              <div className="settings">
                <Appearance theme={theme} onChange={setTheme} />
              </div>
            ) : undefined}
          </SectionPage>
        )}
      </div>
      </FavoriteContext.Provider>
    </MotionConfig>
  );
}

export default App;
