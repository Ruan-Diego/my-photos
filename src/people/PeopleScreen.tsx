import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { thumbnailUrl } from "../viewer/media";
import { filterPeople, initials, itemsLabel, peopleSubtitle, personHash, type PersonCard } from "./people";

function usePeople(): [PersonCard[] | null, string | null] {
  const [people, setPeople] = useState<PersonCard[] | null>(null);
  const [failure, setFailure] = useState<string | null>(null);
  useEffect(() => {
    invoke<PersonCard[]>("people_list")
      .then(setPeople)
      .catch((error) => setFailure(String(error)));
  }, []);
  return [people, failure];
}

function Person({ person }: { person: PersonCard }) {
  const [broken, setBroken] = useState(false);
  return (
    <a className="person" href={personHash(person.id)} draggable={false}>
      <span className="person-cover" aria-hidden="true">
        {person.cover !== null && !broken ? (
          <img src={thumbnailUrl(person.cover)} alt="" loading="lazy" decoding="async" draggable={false} onError={() => setBroken(true)} />
        ) : (
          initials(person.name)
        )}
      </span>
      <span className="person-text">
        <strong>{person.name}</strong>
        <span>{itemsLabel(person.count)}</span>
      </span>
    </a>
  );
}

/** F23: pessoas marcadas no Google Fotos (artboard `People`); cada uma abre a busca filtrada. */
export function PeopleScreen() {
  const [people, failure] = usePeople();
  const [text, setText] = useState("");
  const shown = people === null ? [] : filterPeople(people, text);

  return (
    <main className="page people" aria-labelledby="people-title">
      <header className="page-header people-header">
        <div className="people-heading">
          <h1 id="people-title">Pessoas</h1>
          <span>{failure !== null ? "Não foi possível ler as pessoas" : people === null ? " " : peopleSubtitle(people.length)}</span>
        </div>
        {people !== null && people.length > 0 ? (
          <label className="people-filter">
            <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" aria-hidden="true">
              <path d="M11 18a7 7 0 1 0 0-14 7 7 0 0 0 0 14zM20 20l-4-4" />
            </svg>
            <span className="visually-hidden">Filtrar pessoas</span>
            <input
              type="text"
              value={text}
              placeholder="Filtrar por nome"
              onChange={(event) => setText(event.target.value)}
              onKeyDown={(event) => {
                if (event.key === "Escape" && text !== "") {
                  event.preventDefault();
                  setText("");
                }
              }}
            />
          </label>
        ) : null}
      </header>

      <div className="page-body">
        {failure !== null ? (
          <div className="page-empty">
            <p className="page-empty-title">Não foi possível ler as pessoas</p>
            <p className="page-empty-detail">{failure}</p>
          </div>
        ) : people !== null && people.length === 0 ? (
          <div className="page-empty">
            <p className="page-empty-title">Ninguém marcado no Google Fotos</p>
            <p className="page-empty-detail">As pessoas aparecem aqui quando os JSONs do Takeout trazem quem está em cada foto.</p>
          </div>
        ) : people !== null && shown.length === 0 ? (
          <div className="page-empty">
            <p className="page-empty-title">Ninguém com esse nome</p>
            <p className="page-empty-detail">Confira a grafia ou apague o filtro.</p>
          </div>
        ) : (
          <section className="people-grid" aria-label="Pessoas">
            {shown.map((person) => (
              <Person key={person.id} person={person} />
            ))}
          </section>
        )}
      </div>
    </main>
  );
}
