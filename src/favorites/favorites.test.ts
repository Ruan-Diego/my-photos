import assert from "node:assert/strict";
import { test } from "node:test";
import { FAVORITE, VIDEO, type TimelineData } from "../timeline/timeline.ts";
import { backHash, backLabel, openedId, viewerHash } from "../viewer/navigation.ts";
import { emptyFavorites, favoriteCount, favoritesByYear, favoritesSubtitle, withFavorite } from "./favorites.ts";

// 2022-08-10, 2022-01-02, 2021-05-01, sem data; mais novo primeiro, como o comando `timeline`.
const DATA: TimelineData = {
  ids: [1, 2, 3, 4, 5],
  times: [1660138620, 1641117600, 1641117000, 1619870400, null],
  offsets: [0, 0, 0, 0, 0],
  ratios: [1, 1, 1, 1, 1],
  durations: [null, 1000, null, null, null],
  flags: [FAVORITE, FAVORITE | VIDEO, 0, FAVORITE, FAVORITE],
};

const ids = (groups: ReturnType<typeof favoritesByYear>) => groups.map((year) => [year.title, year.items.map((item) => item.id)]);

test("FAVUI-01 subtitle and empty texts read like the canvas", () => {
  assert.equal(favoritesSubtitle(96), "96 itens · marcados só no app, o arquivo original não muda");
  assert.equal(favoritesSubtitle(1), "1 item · marcados só no app, o arquivo original não muda");
  assert.equal(favoritesSubtitle(1800), "1.800 itens · marcados só no app, o arquivo original não muda");
  assert.equal(favoriteCount(DATA), 4);
  assert.equal(emptyFavorites("all", 0).title, "Nenhum favorito ainda");
  assert.equal(emptyFavorites("videos", 3).title, "Nenhum vídeo favorito");
});

test("FAVUI-02 favorites grouped by year, newest first, with the type filter", () => {
  assert.deepEqual(ids(favoritesByYear(DATA, "all")), [
    ["2022", [1, 2]],
    ["2021", [4]],
    ["Sem data", [5]],
  ]);
  assert.deepEqual(ids(favoritesByYear(DATA, "videos")), [["2022", [2]]]);
  assert.deepEqual(ids(favoritesByYear(DATA, "photos")), [
    ["2022", [1]],
    ["2021", [4]],
    ["Sem data", [5]],
  ]);
});

test("FAVUI-03 marking changes one flag; an unmarked item can stay while the viewer is open", () => {
  const unmarked = withFavorite(DATA, 1, false);
  assert.equal(unmarked.flags[0], 0);
  assert.equal(DATA.flags[0], FAVORITE, "the original data is not touched");
  assert.equal(withFavorite(unmarked, 1, false), unmarked, "no change, same object");
  assert.equal(withFavorite(DATA, 2, false).flags[1], VIDEO, "the video flag stays");
  assert.equal(withFavorite(DATA, 3, true).flags[2], FAVORITE);
  assert.equal(withFavorite(DATA, 99, true), DATA);
  assert.deepEqual(ids(favoritesByYear(unmarked, "all")).slice(0, 1), [["2022", [2]]]);
  assert.deepEqual(ids(favoritesByYear(unmarked, "all", new Set([1]))).slice(0, 1), [["2022", [1, 2]]]);
  assert.equal(favoritesByYear(unmarked, "all", new Set([1]))[0].items[0].favorite, false, "kept, but shown as unmarked");
});

test("FAVUI-04 the viewer opened from Favoritos goes back there", () => {
  assert.equal(openedId("#/favoritos/12", "favoritos"), 12);
  assert.equal(openedId("#/favoritos/12"), null);
  assert.equal(viewerHash(12, "favoritos"), "#/favoritos/12");
  assert.equal(backHash("favoritos"), "#/favoritos");
  assert.equal(backLabel("favoritos"), "Voltar para Favoritos");
});
