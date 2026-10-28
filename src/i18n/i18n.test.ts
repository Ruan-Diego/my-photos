import assert from "node:assert/strict";
import { readdirSync, readFileSync, statSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { test } from "node:test";
import { formatCount, list, LOCALES, placeholders, t, type Key } from "./i18n.ts";
import { ptBR } from "./pt-BR.ts";

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
