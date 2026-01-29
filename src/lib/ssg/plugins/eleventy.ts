import type { SSGPlugin } from "../contract";
import { eleventySchema } from "../schema";
import { validateData } from "../../validation/engine";

const getTodayString = (): string => {
  const now = new Date();
  const year = now.getFullYear();
  const month = String(now.getMonth() + 1).padStart(2, "0");
  const day = String(now.getDate()).padStart(2, "0");
  return `${year}-${month}-${day}`;
};

export const eleventyPlugin: SSGPlugin = {
  id: "eleventy",
  label: "Eleventy",
  formats: ["yaml", "toml"],
  getSchema: () => eleventySchema,
  getDefaults: () => ({ draft: false, date: getTodayString() }),
  validate: (data) => validateData(eleventySchema, data),
  serialize: () => {
    throw new Error("serialize not implemented");
  },
};
