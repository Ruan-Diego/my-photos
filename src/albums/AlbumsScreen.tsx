import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { thumbnailUrl } from "../viewer/media";
import { albumHash, albumMeta, loadOrder, ORDER_KEY, ORDERS, sortAlbums, type AlbumCard, type AlbumOrder } from "./albums";
import { t } from "../i18n/i18n";

function useAlbums(): [AlbumCard[] | null, string | null] {
  const [albums, setAlbums] = useState<AlbumCard[] | null>(null);
  const [failure, setFailure] = useState<string | null>(null);
  useEffect(() => {
    invoke<AlbumCard[]>("albums_list")
      .then(setAlbums)
      .catch((error) => setFailure(String(error)));
  }, []);
  return [albums, failure];
}

function storage(): Storage | null {
  try {
    return window.localStorage;
  } catch {
    return null;
  }
}

/** Uma foto da pilha; some se a miniatura falhar. */
function Layer({ id, className }: { id: number | undefined; className: string }) {
  const [broken, setBroken] = useState(false);
  return (
    <span className={className} aria-hidden="true">
      {id !== undefined && !broken ? (
        <img src={thumbnailUrl(id)} alt="" loading="lazy" decoding="async" draggable={false} onError={() => setBroken(true)} />
      ) : null}
    </span>
  );
}

function Album({ album }: { album: AlbumCard }) {
  const [front, middle, back] = album.covers;
  return (
    <a className="album" href={albumHash(album.id)} draggable={false}>
      <span className="album-stack">
        <Layer id={back ?? front} className="album-layer album-layer-back" />
        <Layer id={middle ?? front} className="album-layer album-layer-middle" />
        <Layer id={front} className="album-cover" />
      </span>
      <span className="album-text">
        <strong>{album.name}</strong>
        <span>{albumMeta(album)}</span>
      </span>
    </a>
  );
}

/** F25: álbuns do Takeout (artboard `Albums`); cada um abre as fotos dele na busca. */
export function AlbumsScreen() {
  const [albums, failure] = useAlbums();
  const [order, setOrder] = useState<AlbumOrder>(() => loadOrder(storage()));
  const choose = (value: AlbumOrder) => {
    setOrder(value);
    try {
      storage()?.setItem(ORDER_KEY, value);
    } catch {
      // Sem armazenamento: só não fica para a próxima vez.
    }
  };

  return (
    <main className="page albums" aria-labelledby="albums-title">
      <header className="page-header">
        <div className="people-heading">
          <h1 id="albums-title">{t("section.albums")}</h1>
          <span>{t("albums.subtitle")}</span>
        </div>
        {albums !== null && albums.length > 0 ? (
          <label className="albums-order">
            {t("albums.order")}
            <select value={order} onChange={(event) => choose(event.target.value as AlbumOrder)}>
              {ORDERS.map((option) => (
                <option key={option.value} value={option.value}>
                  {option.label}
                </option>
              ))}
            </select>
          </label>
        ) : null}
      </header>
      <div className="page-body">
        {failure !== null ? (
          <div className="page-empty">
            <p className="page-empty-title">{t("albums.readFailed")}</p>
            <p className="page-empty-detail">{failure}</p>
          </div>
        ) : albums !== null && albums.length === 0 ? (
          <div className="page-empty">
            <p className="page-empty-title">{t("albums.emptyTitle")}</p>
            <p className="page-empty-detail">
              {t("albums.emptyDetail")}
            </p>
          </div>
        ) : albums !== null ? (
          <section className="albums-grid" aria-label={t("section.albums")}>
            {sortAlbums(albums, order).map((album) => (
              <Album key={album.id} album={album} />
            ))}
          </section>
        ) : null}
      </div>
    </main>
  );
}
