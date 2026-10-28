import assert from "node:assert/strict";
import { readdirSync, readFileSync, statSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { test } from "node:test";
import { formatCount, list, LOCALES, placeholders, t, type Key } from "./i18n.ts";
import { ptBR } from "./pt-BR.ts";
import { leftovers, looksLikeText } from "./scan.ts";

test("I18N-01 messages fill in numbers and pick the plural", () => {
  assert.equal(t("count.items", { n: 0 }), "0 itens");
  assert.equal(t("count.items", { n: 1 }), "1 item");
  assert.equal(t("count.items", { n: 12345 }), "12.345 itens");
  assert.equal(t("count.thumbnailsLeft", { n: 1 }), "falta 1 miniatura");
  assert.equal(t("section.settings"), "Configurações");
  assert.equal(formatCount(6130), "6.130");
  assert.equal(list("calendar.months").length, 12);
  assert.equal(list("calendar.weekdays").length, 7);
});

test("I18N-02 every language has every key, with the same placeholders", () => {
  for (const [code, messages] of Object.entries(LOCALES)) {
    for (const key of Object.keys(ptBR) as Key[]) {
      const message = messages[key];
      assert.ok(message !== undefined, `${code}: falta ${key}`);
      assert.deepEqual(placeholders(message), placeholders(ptBR[key]), `${code}: ${key}`);
      assert.equal(typeof message, typeof ptBR[key], `${code}: ${key} (plural)`);
    }
  }
});

test("I18N-03 number and date formats go through the i18n module", () => {
  const offenders: string[] = [];
  const walk = (folder: string) => {
    for (const name of readdirSync(folder)) {
      const path = join(folder, name);
      if (statSync(path).isDirectory()) {
        if (name !== "i18n") {
          walk(path);
        }
      } else if (/\.tsx?$/.test(name) && !name.endsWith(".test.ts")) {
        const text = readFileSync(path, "utf8");
        if (/(Intl\.\w+|toLocale\w*String)\(\s*"[a-z]{2}(-[A-Z]{2})?"/.test(text)) {
          offenders.push(path);
        }
      }
    }
  };
  walk(fileURLToPath(new URL("..", import.meta.url)));
  assert.deepEqual(offenders, []);
});

test("I18N-06 nenhum texto de tela fora do catálogo", () => {
  const root = fileURLToPath(new URL("..", import.meta.url));
  const found: string[] = [];
  const walk = (folder: string) => {
    for (const name of readdirSync(folder)) {
      const path = join(folder, name);
      if (statSync(path).isDirectory()) {
        if (name !== "i18n") {
          walk(path);
        }
      } else if (/\.tsx?$/.test(name) && !name.endsWith(".test.ts")) {
        for (const item of leftovers(path.slice(root.length), readFileSync(path, "utf8"))) {
          found.push(`${item.file}:${item.line} ${item.text}`);
        }
      }
    }
  };
  walk(root);
  assert.deepEqual(found, []);
});

test("I18N-07 o detector acha texto e ignora código", () => {
  const sample = [
    'const a = <p className="x y">Nenhuma foto</p>;',
    'const b = <button aria-label="Fechar aviso" />;',
    "const c = `desde ${d}`;",
    'const d = n === 1 ? "1 nota" : x;',
    'const e = invoke("folder_tree");',
    'const f = <svg viewBox="0 0 24 24"><path d="M4 4h7" /></svg>;',
    'if (key === "Escape") {}',
    'const g = <span>{t("section.trash")}</span>;',
    'const h = <span className="side-heading">My Photos</span>;',
    "const i = { flex: `${grow} 1 ${size}px` };",
  ].join("\n");
  assert.deepEqual(
    leftovers("sample.tsx", sample).map((item) => item.line),
    [1, 2, 3, 4],
  );
  assert.equal(looksLikeText("Tamanho"), true);
  assert.equal(looksLikeText("Apagando…"), true);
  assert.equal(looksLikeText("folder_tree"), false);
  assert.equal(looksLikeText("#/fotos"), false);
});
