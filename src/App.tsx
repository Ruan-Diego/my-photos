import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { MotionConfig } from "motion/react";
import { linkedMotionLibraries } from "./motion/libraries";
import { Appearance } from "./shell/Appearance";
import { diskCard, type DiskCard, type LibrarySummary } from "./shell/library";
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

function App() {
  const active = useSection();
  const [theme, setTheme] = useTheme();
  const [disk, setDisk] = useState<DiskCard | null>(null);

  useEffect(() => {
    let live = true;
    linkedMotionLibraries();
    invoke<LibrarySummary>("library_summary")
      .then((summary) => live && setDisk(diskCard(summary)))
      .catch(() => live && setDisk(diskCard(null)));
    return () => {
      live = false;
    };
  }, []);

  const section = sectionById(active);
  return (
    <MotionConfig reducedMotion="user">
      <div className="shell">
        <Sidebar active={active} disk={disk} />
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
