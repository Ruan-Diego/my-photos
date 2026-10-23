import assert from "node:assert/strict";
import { test } from "node:test";
import {
  basis,
  calendar,
  dayLabel,
  duration,
  emptyText,
  FAVORITE,
  GAP,
  group,
  layout,
  rowHeight,
  rows,
  VIDEO,
  visible,
  yearAt,
  years,
  type Item,
  type TimelineData,
} from "./timeline.ts";

// 2022-08-10T14:37Z, 2022-08-10T09:00Z, 2022-08-11T23:30Z (Lisboa +01:00 → dia 12), 2021-07-23T18:00Z.
const DATA: TimelineData = {
  ids: [4, 3, 2, 1, 9],
  times: [1660260600, 1660142220, 1660122000, 1627063200, null],
  offsets: [60, 0, 0, 0, null],
  ratios: [1.5, 0.75, 1.78, 1, 1.33],
  durations: [null, null, 138000, null, null],
  flags: [0, FAVORITE, VIDEO, 0, 0],
};

test("TL-01 groups by month and day in the photo's own time zone", () => {
  const months = group(DATA, "all");
  assert.deepEqual(
    months.map((month) => [month.title, month.days.map((day) => [day.label, day.items.map((item) => item.id)])]),
    [
      ["Agosto de 2022", [["Sex, 12 de ago", [4]], ["Qua, 10 de ago", [3, 2]]]],
      ["Julho de 2021", [["Sex, 23 de jul", [1]]]],
      ["Sem data", [["Data não encontrada no arquivo nem no JSON", [9]]]],
    ],
  );
  assert.deepEqual(years(months), [2022, 2021]);
  assert.deepEqual(months[0].days[1].items[1], { id: 2, ratio: 1.78, video: true, favorite: false, duration: 138000, time: 1660122000, offset: 0 });
});

test("TL-02 filters keep photos, videos or favorites", () => {
  const ids = (filter: Parameters<typeof group>[1]) => group(DATA, filter).flatMap((month) => month.days.flatMap((day) => day.items.map((item) => item.id)));
  assert.deepEqual(ids("photos"), [4, 3, 1, 9]);
  assert.deepEqual(ids("videos"), [2]);
  assert.deepEqual(ids("favorites"), [3]);
  assert.deepEqual(emptyText("videos", 5).title, "Nenhum vídeo");
  assert.deepEqual(emptyText("all", 0).title, "Nenhuma foto ainda");
});

test("TL-03 labels and durations read like the canvas", () => {
  assert.equal(dayLabel(calendar(1660142220, 0)), "Qua, 10 de ago");
  assert.equal(duration(138000), "2:18");
  assert.equal(duration(42000), "0:42");
  assert.equal(duration(3723000), "1:02:03");
  assert.equal(rowHeight(3), 176);
  assert.equal(rowHeight(9), 296);
});

function item(id: number, ratio: number): Item {
  return { id, ratio, video: false, favorite: false, duration: null, time: null, offset: null };
}

test("TL-04 rows wrap like the canvas flex-wrap", () => {
  // 176 px: 1.5 → 264, 0.75 → 132, 1 → 176.
  const items = [item(1, 1.5), item(2, 0.75), item(3, 1), item(4, 1.5), item(5, 0.2), item(6, 9)];
  assert.equal(basis(0.2, 176), 88, "narrow photos are clamped");
  assert.equal(basis(9, 176), 528, "panoramas are clamped");
  const width = 264 + GAP + 132 + GAP + 176;
  assert.deepEqual(
    rows(items, width, 176).map((row) => row.map((tile) => tile.id)),
    [[1, 2, 3], [4, 5], [6]],
  );
  assert.deepEqual(rows([item(1, 3)], 100, 176).map((row) => row.length), [1], "a tile wider than the row still gets one");
});

test("TL-05 only the lines on screen are rendered", () => {
  const many: TimelineData = {
    ids: Array.from({ length: 400 }, (_, index) => index),
    times: Array.from({ length: 400 }, (_, index) => 1660142220 - index * 86400 * 3),
    offsets: Array.from({ length: 400 }, () => 0),
    ratios: Array.from({ length: 400 }, () => 1.5),
    durations: Array.from({ length: 400 }, () => null),
    flags: Array.from({ length: 400 }, () => 0),
  };
  const { lines, height, yearTops } = layout(group(many, "all"), 1000, 176);
  assert.ok(height > 20000);
  const [start, end] = visible(lines, 10000, 800);
  assert.ok(end - start < 40, `${end - start} lines`);
  assert.ok(lines[start].top + lines[start].height > 10000 - 800);
  assert.ok(lines[end - 1].top < 10000 + 1600);
  for (let index = 1; index < lines.length; index += 1) {
    assert.ok(lines[index].top >= lines[index - 1].top + lines[index - 1].height, "no overlap");
  }
  assert.equal(yearAt(lines, 0), 2022);
  const top2020 = yearTops.get(2020);
  assert.ok(top2020 !== undefined);
  assert.equal(yearAt(lines, top2020 + 10), 2020);
  assert.deepEqual([...yearTops.keys()], [2022, 2021, 2020, 2019]);
});
