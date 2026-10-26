import assert from "node:assert/strict";
import { test } from "node:test";
import { chips, dateChip, folderLabel, invalidDate, isBlank, NO_FILTERS, parseDate, removeChip, resultsTitle, toQuery } from "./query.ts";

const PEOPLE = [{ id: 7, name: "Ana", count: 3 }];

test("SRCHUI-01 the filters become the F14 query", () => {
  assert.equal(parseDate("01/08/2022"), "2022-08-01");
  assert.equal(parseDate("1/8/2022"), "2022-08-01");
  assert.equal(parseDate("31/02/2022"), null);
  assert.equal(parseDate("ontem"), null);
  assert.ok(invalidDate("32/01/2022"));
  assert.ok(!invalidDate(" "));
  assert.deepEqual(toQuery(" lisboa ", { ...NO_FILTERS, from: "01/08/2022", until: "31/08/2022", videos: false, people: [7], folder: "Takeout/Viagem" }), {
    text: "lisboa",
    mediaType: "photo",
    from: "2022-08-01",
    until: "2022-09-01",
    people: [7],
    folder: "Takeout/Viagem",
    favoritesOnly: false,
  });
  assert.equal(toQuery("", NO_FILTERS).mediaType, null);
  assert.ok(isBlank("  ", NO_FILTERS));
  assert.ok(!isBlank("", { ...NO_FILTERS, favoritesOnly: true }));
  assert.ok(!isBlank("", { ...NO_FILTERS, photos: false, videos: false }));
});

test("SRCHUI-02 chips read like the canvas and remove one filter each", () => {
  const filters = { ...NO_FILTERS, from: "01/08/2022", until: "31/08/2022", people: [7], folder: "Takeout/Google Fotos/Viagem" };
  assert.deepEqual(chips(filters, PEOPLE), [
    { key: "date", label: "ago 2022" },
    { key: "person:7", label: "Ana" },
    { key: "folder", label: "Google Fotos\\Viagem" },
  ]);
  assert.equal(dateChip({ ...NO_FILTERS, from: "05/08/2022", until: "31/08/2022" }), "05/08/2022 a 31/08/2022");
  assert.equal(dateChip({ ...NO_FILTERS, from: "05/08/2022" }), "desde 05/08/2022");
  assert.deepEqual(removeChip(filters, "person:7").people, []);
  assert.deepEqual(removeChip(filters, "date").from, "");
  assert.equal(folderLabel("Ruan/2020"), "Ruan\\2020");
  assert.equal(resultsTitle(48), "48 resultados");
  assert.equal(resultsTitle(1), "1 resultado");
  assert.equal(resultsTitle(1234), "1.234 resultados");
  assert.equal(resultsTitle(0), "Nenhum resultado");
});
