import assert from "node:assert/strict";
import { test } from "node:test";
import { diskCard, type LibrarySummary } from "./library.ts";
import { hashFor, SECTIONS, sectionFromHash } from "./sections.ts";
import { loadTheme, parseTheme, saveTheme, THEME_KEY, themeAttribute } from "../theme/theme.ts";

test("SHELL-02 sidebar order and groups follow the canvas", () => {
  assert.deepEqual(
    SECTIONS.filter((s) => s.group === "main").map((s) => s.label),
    ["Fotos", "Pastas", "Álbuns", "Pessoas", "Locais", "Favoritos"],
  );
  assert.deepEqual(
    SECTIONS.filter((s) => s.group === "care").map((s) => s.label),
    ["Duplicatas", "Problemas", "Lixeira", "Configurações"],
  );
});

test("SHELL-03 every section has its own hash and comes back from it", () => {
  const hashes = new Set(SECTIONS.map((s) => hashFor(s.id)));
  assert.equal(hashes.size, SECTIONS.length);
  for (const section of SECTIONS) {
    assert.equal(sectionFromHash(hashFor(section.id)), section.id);
  }
  assert.equal(sectionFromHash("#/Pessoas"), "people");
  assert.equal(sectionFromHash("#/pessoas/12"), "people");
  assert.equal(sectionFromHash("#/buscar/12"), "search");
});

test("SHELL-03 empty or unknown hash opens Fotos", () => {
  for (const hash of ["", "#", "#/", "#/nada", "#/../pastas"]) {
    assert.equal(sectionFromHash(hash), "timeline");
  }
});

test("SHELL-04 theme choice: only light and dark are fixed; anything else follows Windows", () => {
  assert.equal(parseTheme("light"), "light");
  assert.equal(parseTheme("dark"), "dark");
  for (const stored of [null, undefined, "", "system", "escuro", "DARK"]) {
    assert.equal(parseTheme(stored), "system");
  }
  assert.equal(themeAttribute("system"), undefined);
  assert.equal(themeAttribute("dark"), "dark");
});

test("SHELL-04 theme choice is stored, and following Windows clears it", () => {
  const map = new Map<string, string>();
  const store = {
    getItem: (key: string) => map.get(key) ?? null,
    setItem: (key: string, value: string) => void map.set(key, value),
    removeItem: (key: string) => void map.delete(key),
  };
  saveTheme("dark", store);
  assert.equal(map.get(THEME_KEY), "dark");
  assert.equal(loadTheme(store), "dark");
  saveTheme("system", store);
  assert.equal(map.has(THEME_KEY), false);
  assert.equal(loadTheme(store), "system");

  const broken = {
    getItem: () => { throw new Error("bloqueado"); },
    setItem: () => { throw new Error("bloqueado"); },
    removeItem: () => { throw new Error("bloqueado"); },
  };
  assert.equal(loadTheme(broken), "system");
  saveTheme("light", broken);
  assert.equal(loadTheme(null), "system");
});

const connected: LibrarySummary = {
  connected: true,
  rootPath: "E:\\",
  volumeLabel: "FOTOS",
  mediaCount: 4247,
  lastIndexedAt: "2026-10-15T17:02:00Z",
  problem: null,
};

test("SHELL-01 connected disk shows drive, count and when it was indexed", () => {
  const now = new Date("2026-10-15T20:00:00Z");
  const card = diskCard(connected, now);
  assert.equal(card.tone, "ok");
  assert.equal(card.title, "E:\\ conectado");
  assert.match(card.detail, /^4\.247 mídias · atualizado (às|em) /);
  assert.equal(card.action, "Procurar arquivos novos");

  assert.equal(diskCard({ ...connected, mediaCount: 1, lastIndexedAt: null }, now).detail, "1 mídia · ainda não indexado");
  assert.equal(diskCard({ ...connected, rootPath: null }, now).title, "Biblioteca conectada");
  assert.equal(diskCard({ ...connected, lastIndexedAt: "lixo" }, now).detail, "4.247 mídias");
});

test("SHELL-01 disconnected disk or failed call shows the warning card", () => {
  const off = diskCard({ ...connected, connected: false, problem: "disco da biblioteca não encontrado" });
  assert.equal(off.tone, "warn");
  assert.equal(off.title, "HD desconectado");
  assert.equal(off.detail, "disco da biblioteca não encontrado");
  assert.equal(diskCard(null).detail, "não foi possível ler a biblioteca");
});
