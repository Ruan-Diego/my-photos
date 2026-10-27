import { createContext, useContext } from "react";

/** Marca (`true`) ou desmarca um item; `null` fora do app com a biblioteca aberta. */
export type ToggleFavorite = (id: number, favorite: boolean) => void;

export const FavoriteContext = createContext<ToggleFavorite | null>(null);

export function useToggleFavorite(): ToggleFavorite | null {
  return useContext(FavoriteContext);
}
