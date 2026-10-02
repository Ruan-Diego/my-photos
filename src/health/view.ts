export const MOTION_LIBRARIES_LINE = "three, gsap, animejs, motion";

export interface AppHealth {
  appName: string;
  schemaVersion: number;
}

export interface HealthScreen {
  ready: string | null;
  schema: string | null;
  libraries: string | null;
  failure: string | null;
}

function failed(message: string): HealthScreen {
  return {
    ready: null,
    schema: null,
    libraries: null,
    failure: `Falha: ${message}`,
  };
}

export function healthView(
  health: AppHealth | null,
  errorMessage: string | null,
): HealthScreen {
  if (errorMessage !== null) {
    return failed(errorMessage);
  }

  if (
    health === null ||
    !Number.isInteger(health.schemaVersion) ||
    health.schemaVersion < 1
  ) {
    return failed("schemaVersion inválido");
  }

  return {
    ready: "My Photos está pronto.",
    schema: `Banco: esquema v${health.schemaVersion}`,
    libraries: MOTION_LIBRARIES_LINE,
    failure: null,
  };
}
