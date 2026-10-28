// Tema claro e escuro (artboard `Tema`): seguir o Windows por padrão, ou fixo.

import { t } from "../i18n/i18n.ts";

export type ThemeChoice = "system" | "light" | "dark";

export const THEME_KEY = "myphotos.theme";

export const THEME_OPTIONS: ReadonlyArray<{ choice: ThemeChoice; label: string }> = [
  { choice: "system", label: t("settings.followWindows") },
  { choice: "light", label: t("settings.themeLight") },
  { choice: "dark", label: t("settings.themeDark") },
];

/** Anything stored that is not a known choice means "follow Windows". */
export function parseTheme(stored: string | null | undefined): ThemeChoice {
  return stored === "light" || stored === "dark" ? stored : "system";
}

/** Value for `data-theme` on <html>; `undefined` removes it so the media query decides. */
export function themeAttribute(choice: ThemeChoice): "light" | "dark" | undefined {
  return choice === "system" ? undefined : choice;
}

interface ThemeStorage {
  getItem(key: string): string | null;
  setItem(key: string, value: string): void;
  removeItem(key: string): void;
}

function storage(): ThemeStorage | null {
  try {
    return globalThis.localStorage ?? null;
  } catch {
    return null;
  }
}

export function loadTheme(store: ThemeStorage | null = storage()): ThemeChoice {
  try {
    return parseTheme(store?.getItem(THEME_KEY));
  } catch {
    return "system";
  }
}

export function saveTheme(choice: ThemeChoice, store: ThemeStorage | null = storage()): void {
  try {
    if (choice === "system") {
      store?.removeItem(THEME_KEY);
    } else {
      store?.setItem(THEME_KEY, choice);
    }
  } catch {
    // Sem armazenamento a escolha vale até fechar o app.
  }
}

export function applyTheme(choice: ThemeChoice, root: HTMLElement = document.documentElement): void {
  const value = themeAttribute(choice);
  if (value === undefined) {
    delete root.dataset.theme;
  } else {
    root.dataset.theme = value;
  }
}
