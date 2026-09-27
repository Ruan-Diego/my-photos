export const MOTION_LIBRARIES_LINE = "three, gsap, animejs, motion";

export interface AppHealth {
  appName: string;
  sqliteOk: boolean;
}

export interface HealthScreen {
  ready: string | null;
  sqlite: string | null;
  libraries: string | null;
  failure: string | null;
}

export function healthView(
  health: AppHealth | null,
  errorMessage: string | null,
): HealthScreen {
  if (errorMessage !== null) {
    return {
      ready: null,
      sqlite: null,
      libraries: null,
      failure: `Falha: ${errorMessage}`,
    };
  }

  if (health === null || health.sqliteOk !== true) {
    return {
      ready: null,
      sqlite: null,
      libraries: null,
      failure: "Falha: sqliteOk inválido",
    };
  }

  return {
    ready: "My Photos está pronto.",
    sqlite: "SQLite: ok",
    libraries: MOTION_LIBRARIES_LINE,
    failure: null,
  };
}
