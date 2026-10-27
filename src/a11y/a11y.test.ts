import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import { contrast, PAIRS, themes } from "./contrast.ts";
import { wrapTarget } from "./focus.ts";
import { tileLabel } from "../timeline/timeline.ts";

test("A11Y-01 every text token pair meets WCAG AA in both themes", () => {
  const css = readFileSync(new URL("../theme/tokens.css", import.meta.url), "utf8");
  const { light, dark } = themes(css);
  assert.ok(dark.get("ground") !== light.get("ground"));
  const failures: string[] = [];
  for (const [name, tokens] of [["claro", light], ["escuro", dark]] as const) {
    for (const [text, background, minimum] of PAIRS) {
      const ratio = contrast(tokens.get(text) ?? "", tokens.get(background) ?? "");
      if (!(ratio >= minimum)) {
        failures.push(`${name}: --${text} sobre --${background} = ${ratio.toFixed(2)}`);
      }
    }
  }
  assert.deepEqual(failures, []);
  // O branco do canvas sobre o laranja escuro (pinos do mapa) ficava em 2,97.
  assert.ok(contrast("#ffffff", "#e07a4c") < 4.5);
});

test("A11Y-02 Tab wraps inside a modal dialog", () => {
  assert.equal(wrapTarget(2, 3, false), 0);
  assert.equal(wrapTarget(0, 3, true), 2);
  assert.equal(wrapTarget(1, 3, false), null);
  assert.equal(wrapTarget(-1, 3, false), 0);
  assert.equal(wrapTarget(-1, 3, true), 2);
  assert.equal(wrapTarget(-1, 0, false), null);
});

test("A11Y-03 photo tiles say what they are and when", () => {
  const base = { id: 1, ratio: 1, video: false, favorite: false, duration: null, time: Date.UTC(2022, 7, 10, 17, 37) / 1000, offset: -180 };
  assert.equal(tileLabel(base), "Foto de 10 de agosto de 2022 às 14:37");
  assert.equal(tileLabel({ ...base, video: true, duration: 138000, favorite: true }), "Vídeo de 10 de agosto de 2022 às 14:37, 2:18, favorito");
  assert.equal(tileLabel({ ...base, favorite: true }), "Foto de 10 de agosto de 2022 às 14:37, favorita");
  assert.equal(tileLabel({ ...base, time: null }), "Foto sem data");
});
