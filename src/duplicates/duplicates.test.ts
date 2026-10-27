import assert from "node:assert/strict";
import { test } from "node:test";
import { FAVORITE, VIDEO, type TimelineData } from "../timeline/timeline.ts";
import { copyMeta, emptyDuplicates, filterGroups, folderLabel, groupSaving, groupTitle, kindFilters, viewerItems, type Group } from "./duplicates.ts";

const copy = (id: number, path: string, sizeBytes: number, width = 4032, height = 3024) => ({
  id,
  path,
  sizeBytes,
  width,
  height,
  takenAt: "2022-08-10T14:37:00Z",
});
const SAME: Group = {
  kind: "identical",
  copies: [copy(1, "t/Takeout/Google Fotos/Viagem Europa 2022/foto001.jpg", 3250586), copy(2, "t/Takeout/Google Fotos/Fotos de 2022/foto001.jpg", 3250586)],
};
const NEAR: Group = {
  kind: "similar",
  copies: [copy(3, "Viagem/IMG_20220811_101522.jpg", 3984588), copy(4, "Organizadas/Portugal/IMG_20220811_101522(1).jpg", 629146, 1600, 1200)],
};

test("DUPUI-01 filters and titles read like the canvas", () => {
  assert.deepEqual(
    kindFilters([SAME, NEAR, SAME]).map((chip) => chip.label),
    ["Todas · 3", "Idênticas · 2", "Parecidas · 1"],
  );
  assert.deepEqual(filterGroups([SAME, NEAR], "similar"), [NEAR]);
  assert.equal(groupTitle(SAME), "foto001.jpg · 2 cópias");
  assert.equal(groupTitle(NEAR), "IMG_20220811_101522 · 2 versões");
  assert.equal(groupSaving(SAME), "3,1 MB repetidos");
  assert.equal(groupSaving(NEAR), "tamanhos diferentes");
  assert.equal(folderLabel(SAME.copies[0].path), "Google Fotos\\Viagem Europa 2022");
  assert.equal(folderLabel("a.jpg"), "\\");
  assert.equal(copyMeta("identical", SAME.copies[0]), "3,1 MB · 10 ago 2022");
  assert.equal(copyMeta("similar", NEAR.copies[1]), "1600 × 1200 · 614 KB");
});

test("DUPUI-02 the viewer walks the copies once each; similar waits for hashes", () => {
  const data: TimelineData = {
    ids: [4, 3, 2, 1],
    times: [1, 2, 3, 4],
    offsets: [0, 0, 0, 0],
    ratios: [1.33, 1.33, 1.5, 1.5],
    durations: [null, null, null, null],
    flags: [0, FAVORITE, VIDEO, 0],
  };
  const items = viewerItems([SAME, NEAR, SAME], data);
  assert.deepEqual(items.map((item) => item.id), [1, 2, 3, 4]);
  assert.equal(items[1].video, true);
  assert.equal(items[2].favorite, true);
  assert.deepEqual(viewerItems([SAME], null), []);
  assert.equal(emptyDuplicates("similar", 10, 100).title, "Nenhuma parecida ainda");
  assert.equal(emptyDuplicates("all", 100, 100).title, "Nenhuma duplicata");
});
