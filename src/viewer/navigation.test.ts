import assert from "node:assert/strict";
import { test } from "node:test";
import { group, type TimelineData } from "../timeline/timeline.ts";
import {
  clock,
  dateLine,
  fallbackNote,
  fileSize,
  indexOf,
  openedId,
  order,
  positionLine,
  seek,
  SEEK_STEP,
  stripWindow,
  videoLine,
  videoProblem,
  viewerHash,
} from "./navigation.ts";

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

test("VIDEO-01 the player header and clock read like the canvas", () => {
  assert.equal(videoLine({ filename: "VID_20220810_170512.mp4", width: 1920, height: 1080, sizeBytes: 84 * 1024 * 1024 }), "VID_20220810_170512.mp4 · 1920 × 1080 · 84 MB");
  assert.equal(videoLine({ filename: "a.mov", width: null, height: null, sizeBytes: 3.4 * 1024 * 1024 }), "a.mov · 3,4 MB");
  assert.equal(fileSize(512), "512 B");
  assert.equal(fileSize(1.25 * 1024 ** 3), "1,3 GB");
  assert.equal(clock(42.9), "0:42");
  assert.equal(clock(138), "2:18");
  assert.equal(clock(3723), "1:02:03");
  assert.equal(clock(Number.NaN), "0:00");
});

test("VIDEO-02 arrows seek 5 s inside the video", () => {
  assert.equal(seek(42, SEEK_STEP, 138), 47);
  assert.equal(seek(2, -SEEK_STEP, 138), 0);
  assert.equal(seek(136, SEEK_STEP, 138), 138);
  assert.equal(seek(3, SEEK_STEP, Number.NaN), 8);
});

test("VIDEO-03 says why a video does not play", () => {
  assert.equal(videoProblem(null, false), null);
  assert.equal(videoProblem({ playable: true, extension: "mp4" }, false), null);
  assert.match(videoProblem({ playable: true, extension: "mp4" }, true) ?? "", /codec/);
  assert.equal(videoProblem({ playable: false, extension: "mkv" }, false), "Vídeos .MKV ainda não tocam no app. Esta é a capa.");
});
