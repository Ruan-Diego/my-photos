import { t } from "../i18n/i18n.ts";

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
    failure: t("common.error", { message }),
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
    return failed(t("health.invalidSchema"));
  }

  return {
    ready: t("health.ready"),
    schema: t("health.schema", { version: health.schemaVersion }),
    libraries: MOTION_LIBRARIES_LINE,
    failure: null,
  };
}
