import assert from "node:assert/strict";
import { test } from "node:test";
import { group, type TimelineData } from "../timeline/timeline.ts";
import { dateLine, fallbackNote, indexOf, openedId, order, positionLine, stripWindow, viewerHash } from "./viewer.ts";

// 2022-08-10T14:37Z e 09:00Z no mesmo dia (UTC), 2021-07-23T18:00Z, e uma sem data.
const DATA: TimelineData = {
  ids: [3, 2, 1, 9],
  times: [1660142220, 1660122000, 1627063200, null],
  offsets: [0, 0, -180, null],
  ratios: [1.5, 1, 1, 1],
  durations: [null, null, null, null],
  flags: [0, 0, 0, 0],
};

test("VIEW-01 the viewer walks the grid in order, with the place in the day", () => {
  const entries = order(group(DATA, "all"));
  assert.deepEqual(
    entries.map((entry) => [entry.item.id, positionLine(entry)]),
    [
      [3, "1 de 2"],
      [2, "2 de 2"],
      [1, "1 de 1"],
      [9, "1 de 1"],
    ],
  );
  assert.equal(indexOf(entries, 1), 2);
  assert.equal(indexOf(entries, 42), -1);
});

test("VIEW-02 the header date reads like the canvas", () => {
  const entries = order(group(DATA, "all"));
  assert.equal(dateLine(entries[0].item), "Qua, 10 de agosto de 2022 · 14:37");
  assert.equal(dateLine(entries[2].item), "Sex, 23 de julho de 2021 · 15:00", "in the photo's own time zone");
  assert.equal(dateLine(entries[3].item), "Sem data");
});

test("VIEW-03 the strip keeps the current one near the middle", () => {
  assert.deepEqual(stripWindow(0, 5), [0, 5]);
  assert.deepEqual(stripWindow(0, 100), [0, 12]);
  assert.deepEqual(stripWindow(50, 100), [45, 57]);
  assert.deepEqual(stripWindow(99, 100), [88, 100]);
});

test("VIEW-04 the address opens and closes the viewer", () => {
  assert.equal(openedId("#/fotos/1066"), 1066);
  assert.equal(openedId("#/fotos"), null);
  assert.equal(openedId("#/pessoas/3"), null);
  assert.equal(viewerHash(7), "#/fotos/7");
  assert.equal(fallbackNote({ video: false, extension: "heic" }), "Arquivos .HEIC ainda não abrem no app. Esta é a miniatura.");
  assert.match(fallbackNote({ video: true, extension: "mp4" }) ?? "", /F20/);
  assert.equal(fallbackNote(null), null);
});
