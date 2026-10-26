// F19/F20: o que o Rust diz de uma mídia e de onde vêm a miniatura e o original.

import { useEffect, useState } from "react";
import { convertFileSrc, invoke } from "@tauri-apps/api/core";

export interface MediaInfo {
  id: number;
  filename: string;
  relativePath: string;
  extension: string;
  video: boolean;
  /** Foto que o app desenha (JPEG, PNG, GIF, WebP). */
  viewable: boolean;
  /** Vídeo que o app toca (MP4, MOV, WebM). */
  playable: boolean;
  width: number | null;
  height: number | null;
  sizeBytes: number | null;
}

export function thumbnailUrl(id: number): string {
  return convertFileSrc(String(id), "thumb");
}

/** `http://media.localhost/1066` no Windows: o original, pelo protocolo `media` do Rust. */
export function originalUrl(id: number): string {
  return convertFileSrc(String(id), "media");
}

export function useMediaInfo(id: number): MediaInfo | null {
  const [info, setInfo] = useState<MediaInfo | null>(null);
  useEffect(() => {
    let current = true;
    setInfo(null);
    invoke<MediaInfo | null>("media_details", { id })
      .then((found) => {
        if (current) {
          setInfo(found);
        }
      })
      .catch(() => undefined);
    return () => {
      current = false;
    };
  }, [id]);
  return info;
}

export const icon = { fill: "none", stroke: "currentColor", strokeWidth: 1.8, strokeLinecap: "round", strokeLinejoin: "round" } as const;

/** O maior retângulo na proporção da foto ou do vídeo que cabe na área. */
export function fit(ratio: number, width: number, height: number): { width: number; height: number } {
  if (width <= 0 || height <= 0) {
    return { width: 0, height: 0 };
  }
  return width / height > ratio ? { width: height * ratio, height } : { width, height: width / ratio };
}
