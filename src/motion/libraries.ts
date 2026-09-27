import { animate } from "animejs";
import { gsap } from "gsap";
import { motion } from "motion/react";
import { REVISION } from "three";
import { MOTION_LIBRARIES_LINE } from "../health/view";

export function linkedMotionLibraries(): typeof MOTION_LIBRARIES_LINE {
  if (
    REVISION === undefined ||
    gsap === undefined ||
    typeof animate !== "function" ||
    motion === undefined
  ) {
    throw new Error("bibliotecas de movimento ausentes");
  }

  return MOTION_LIBRARIES_LINE;
}
