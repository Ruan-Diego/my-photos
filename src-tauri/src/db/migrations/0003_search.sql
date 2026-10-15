-- F14: full-text search (FTS5). One row per media, rowid = media_items.id. Kept in sync
-- by the triggers below, so every writer (scan, index, future UI edits) updates it.
-- `unicode61 remove_diacritics 2`: "sao paulo" finds "São Paulo"; `_` and `/` split
-- words, so "1234" finds IMG_1234.jpg and a folder name finds what is inside it.
CREATE VIRTUAL TABLE media_search USING fts5(
  path, title, description, people, place, note,
  tokenize = 'unicode61 remove_diacritics 2',
  prefix = '2 3'
);

INSERT INTO media_search (rowid, path, title, description, people, place, note)
SELECT m.id, m.relative_path, m.title, m.description,
       (SELECT group_concat(p.name, ' ') FROM media_people mp JOIN people p ON p.id = mp.person_id
         WHERE mp.media_id = m.id),
       (SELECT l.place_label FROM media_locations l WHERE l.media_id = m.id),
       (SELECT n.note FROM user_notes n WHERE n.media_id = m.id)
FROM media_items m;

CREATE TRIGGER media_search_insert AFTER INSERT ON media_items BEGIN
  INSERT INTO media_search (rowid, path, title, description) VALUES (new.id, new.relative_path, new.title, new.description);
END;

CREATE TRIGGER media_search_update AFTER UPDATE OF relative_path, title, description ON media_items BEGIN
  UPDATE media_search SET path = new.relative_path, title = new.title, description = new.description
   WHERE rowid = new.id;
END;

CREATE TRIGGER media_search_delete AFTER DELETE ON media_items BEGIN
  DELETE FROM media_search WHERE rowid = old.id;
END;

CREATE TRIGGER media_search_people_insert AFTER INSERT ON media_people BEGIN
  UPDATE media_search SET people = (SELECT group_concat(p.name, ' ') FROM media_people mp
      JOIN people p ON p.id = mp.person_id WHERE mp.media_id = new.media_id)
   WHERE rowid = new.media_id;
END;

CREATE TRIGGER media_search_people_delete AFTER DELETE ON media_people BEGIN
  UPDATE media_search SET people = (SELECT group_concat(p.name, ' ') FROM media_people mp
      JOIN people p ON p.id = mp.person_id WHERE mp.media_id = old.media_id)
   WHERE rowid = old.media_id;
END;

CREATE TRIGGER media_search_person_rename AFTER UPDATE OF name ON people BEGIN
  UPDATE media_search SET people = (SELECT group_concat(p.name, ' ') FROM media_people mp
      JOIN people p ON p.id = mp.person_id WHERE mp.media_id = media_search.rowid)
   WHERE rowid IN (SELECT media_id FROM media_people WHERE person_id = new.id);
END;

CREATE TRIGGER media_search_place_insert AFTER INSERT ON media_locations BEGIN
  UPDATE media_search SET place = new.place_label WHERE rowid = new.media_id;
END;

CREATE TRIGGER media_search_place_update AFTER UPDATE OF place_label ON media_locations BEGIN
  UPDATE media_search SET place = new.place_label WHERE rowid = new.media_id;
END;

CREATE TRIGGER media_search_place_delete AFTER DELETE ON media_locations BEGIN
  UPDATE media_search SET place = NULL WHERE rowid = old.media_id;
END;

CREATE TRIGGER media_search_note_insert AFTER INSERT ON user_notes BEGIN
  UPDATE media_search SET note = new.note WHERE rowid = new.media_id;
END;

CREATE TRIGGER media_search_note_update AFTER UPDATE OF note ON user_notes BEGIN
  UPDATE media_search SET note = new.note WHERE rowid = new.media_id;
END;

CREATE TRIGGER media_search_note_delete AFTER DELETE ON user_notes BEGIN
  UPDATE media_search SET note = NULL WHERE rowid = old.media_id;
END;

-- Structured filters next to the text match.
CREATE INDEX idx_media_people_person ON media_people(person_id);
