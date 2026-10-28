import { animate } from "animejs";
import { gsap } from "gsap";
import { motion } from "motion/react";
import { REVISION } from "three";
import { MOTION_LIBRARIES_LINE } from "../health/view";
import { t } from "../i18n/i18n";

export function linkedMotionLibraries(): typeof MOTION_LIBRARIES_LINE {
  if (
    REVISION === undefined ||
    gsap === undefined ||
    typeof animate !== "function" ||
    motion === undefined
  ) {
    throw new Error(t("health.motionMissing"));
  }

  return MOTION_LIBRARIES_LINE;
}
