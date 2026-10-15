import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { MotionConfig } from "motion/react";
import { linkedMotionLibraries } from "./motion/libraries";
import {
  failedCard,
  failureText,
  FINISHED_EVENT,
  indexingCard,
  needsOnboarding,
  PROGRESS_EVENT,
  type IndexingFinished,
  type LibraryFolder,
  type Progress,
} from "./onboarding/flow";
import { Onboarding } from "./onboarding/Onboarding";
import { Appearance } from "./shell/Appearance";
import { diskCard, type LibrarySummary } from "./shell/library";
import { SectionPage } from "./shell/SectionPage";
import { sectionById, sectionFromHash, type SectionId } from "./shell/sections";
import { Sidebar } from "./shell/Sidebar";
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

  useEffect(() => {
    linkedMotionLibraries();
  }, []);

  // A indexação roda no Rust; daqui só se ouve o progresso e o fim.
  useEffect(() => {
    const progressOff = listen<Progress>(PROGRESS_EVENT, (event) => setProgress(event.payload));
    const finishedOff = listen<IndexingFinished>(FINISHED_EVENT, (event) => {
      setProgress(null);
      setIndexError(event.payload.error);
      reloadSummary();
    });
    return () => {
      void progressOff.then((off) => off());
      void finishedOff.then((off) => off());
    };
  }, [reloadSummary]);

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
        setProgress({ stage: "scanning", done: 0, total: 0 });
        window.location.hash = "#/fotos";
      })
      .catch((error) => setStartError(errorText(error)))
      .finally(() => setStarting(false));
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

  const disk =
    progress !== null
      ? indexingCard(summary?.rootPath ?? folder?.rootPath ?? null, progress)
      : indexError !== null
        ? failedCard(indexError)
        : startError !== null
          ? { tone: "warn" as const, title: "A indexação não começou", detail: startError, action: null }
        : diskCard(summary);
  const section = sectionById(active);
  return (
    <MotionConfig reducedMotion="user">
      <div className="shell">
        <Sidebar active={active} disk={disk} onDiskAction={indexError !== null && progress === null ? startIndexing : undefined} />
        <SectionPage key={section.id} section={section}>
          {section.id === "settings" ? (
            <div className="settings">
              <Appearance theme={theme} onChange={setTheme} />
            </div>
          ) : undefined}
        </SectionPage>
      </div>
    </MotionConfig>
  );
}

export default App;
