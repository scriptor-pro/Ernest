import type { SSGPlugin } from "../contract";
import { minimalSchema } from "../schema";
import type { SSGId } from "../../types";
import { validateData } from "../../validation/engine";

const getTodayString = (): string => {
  const now = new Date();
  const year = now.getFullYear();
  const month = String(now.getMonth() + 1).padStart(2, "0");
  const day = String(now.getDate()).padStart(2, "0");
  return `${year}-${month}-${day}`;
};

const createMinimalPlugin = (id: SSGId, label: string): SSGPlugin => ({
  id,
  label,
  formats: ["yaml", "toml"],
  getSchema: () => minimalSchema,
  getDefaults: () => ({ draft: false, date: getTodayString() }),
  validate: (data) => validateData(minimalSchema, data),
  serialize: () => {
    throw new Error("serialize not implemented");
  },
});

export const hugoPlugin = createMinimalPlugin("hugo", "Hugo");
export const jekyllPlugin = createMinimalPlugin("jekyll", "Jekyll");
export const gatsbyPlugin = createMinimalPlugin("gatsby", "Gatsby");
export const astroPlugin = createMinimalPlugin("astro", "Astro");
