import assert from "node:assert/strict";
import { test } from "node:test";
import { albumFromHash, albumHash, albumMeta, loadOrder, sortAlbums, type AlbumCard } from "./albums.ts";

const album = (id: number, name: string, count: number, first: string | null, last = first): AlbumCard => ({
  id,
  name,
  count,
  first,
  last,
  covers: [],
});

const TRIP = album(1, "Viagem Europa 2022", 124, "2022-08-10T10:00:00Z", "2022-08-20T10:00:00Z");
const YEAR = album(2, "Fotos de 2022", 1204, "2022-01-02T10:00:00Z", "2022-12-30T10:00:00Z");
const OLD = album(3, "aniversário", 86, "2019-07-01T10:00:00Z");
const UNDATED = album(4, "Escaneadas", 12, null);

test("ALBUI-01 meta and order read like the canvas", () => {
  assert.equal(albumMeta(TRIP), "124 itens · ago 2022");
  assert.equal(albumMeta(YEAR), "1.204 itens · 2022");
  assert.equal(albumMeta(UNDATED), "12 itens");
  assert.equal(albumMeta(album(5, "x", 1, "2015-01-01T00:00:00Z", "2024-01-01T00:00:00Z")), "1 item · 2015–2024");
  const all = [OLD, UNDATED, TRIP, YEAR];
  assert.deepEqual(sortAlbums(all, "recent").map((a) => a.id), [2, 1, 3, 4]);
  assert.deepEqual(sortAlbums(all, "name").map((a) => a.id), [3, 4, 2, 1]);
  assert.deepEqual(sortAlbums(all, "size").map((a) => a.id), [2, 1, 3, 4]);
  assert.equal(loadOrder({ getItem: () => "name" }), "name");
  assert.equal(loadOrder({ getItem: () => "lixo" }), "recent");
  assert.equal(loadOrder({ getItem: () => { throw new Error("bloqueado"); } }), "recent");
  assert.equal(loadOrder(null), "recent");
});

test("ALBUI-02 an album opens the search", () => {
  assert.equal(albumHash(5), "#/buscar?album=5");
  assert.equal(albumFromHash("#/buscar?album=5"), 5);
  assert.equal(albumFromHash("#/buscar?pessoa=5"), null);
});
