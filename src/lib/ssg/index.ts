import type { SSGId } from "../types";
import type { SSGPlugin } from "./contract";
import {
  astroPlugin,
  gatsbyPlugin,
  hugoPlugin,
  jekyllPlugin,
} from "./plugins/minimal";
import { eleventyPlugin } from "./plugins/eleventy";

export const ssgPlugins: Record<SSGId, SSGPlugin> = {
  eleventy: eleventyPlugin,
  hugo: hugoPlugin,
  jekyll: jekyllPlugin,
  gatsby: gatsbyPlugin,
  astro: astroPlugin,
};

export const getSsgPlugin = (id: SSGId): SSGPlugin => ssgPlugins[id];
