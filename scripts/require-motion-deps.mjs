import { readFileSync } from "node:fs";

const pkg = JSON.parse(readFileSync(new URL("../package.json", import.meta.url), "utf8"));
const required = ["three", "gsap", "animejs", "motion"];
const dependencies = pkg.dependencies ?? {};
const missing = required.filter((name) => dependencies[name] === undefined);

if (missing.length > 0) {
  console.error(`dependências ausentes: ${missing.join(", ")}`);
  process.exit(1);
}
