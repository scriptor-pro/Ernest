import type { FrontmatterFormat } from "../types";

export type ParsedFrontmatter = {
  data: Record<string, unknown>;
  body: string;
  format: FrontmatterFormat | null;
  raw: string | null;
};

type FrontmatterSplit = {
  format: FrontmatterFormat;
  raw: string;
  body: string;
} | null;

const detectFrontmatter = (content: string): FrontmatterSplit => {
  const trimmed = content.trimStart();
  const startsWithYaml =
    trimmed.startsWith("---\n") || trimmed.startsWith("---\r\n");
  const startsWithToml =
    trimmed.startsWith("+++\n") || trimmed.startsWith("+++\r\n");
  if (!startsWithYaml && !startsWithToml) {
    return null;
  }

  const delimiter = startsWithYaml ? "---" : "+++";
  const format: FrontmatterFormat = startsWithYaml ? "yaml" : "toml";
  const lines = trimmed.split(/\r?\n/);
  if (lines[0] !== delimiter) {
    return null;
  }

  let endIndex = -1;
  for (let i = 1; i < lines.length; i += 1) {
    if (lines[i] === delimiter) {
      endIndex = i;
      break;
    }
  }

  if (endIndex === -1) {
    return null;
  }

  const raw = lines.slice(1, endIndex).join("\n");
  const body = lines.slice(endIndex + 1).join("\n");
  return { format, raw, body };
};

const parseScalar = (value: string): unknown => {
  const trimmed = value.trim();
  if (trimmed === "") {
    return "";
  }
  if (trimmed === "true") {
    return true;
  }
  if (trimmed === "false") {
    return false;
  }
  const numberValue = Number(trimmed);
  if (!Number.isNaN(numberValue) && /^-?\d+(?:\.\d+)?$/.test(trimmed)) {
    return numberValue;
  }
  if (
    (trimmed.startsWith('"') && trimmed.endsWith('"')) ||
    (trimmed.startsWith("'") && trimmed.endsWith("'"))
  ) {
    return trimmed.slice(1, -1);
  }
  return trimmed;
};

const parseYaml = (raw: string): Record<string, unknown> => {
  const data: Record<string, unknown> = {};
  const lines = raw.split(/\r?\n/);
  let currentKey: string | null = null;
  let currentList: string[] | null = null;

  const commitList = () => {
    if (currentKey && currentList) {
      data[currentKey] = currentList;
    }
    currentKey = null;
    currentList = null;
  };

  for (const line of lines) {
    if (line.trim() === "") {
      continue;
    }

    if (line.trim().startsWith("- ")) {
      if (!currentKey) {
        continue;
      }
      const item = line.trim().slice(2);
      if (!currentList) {
        currentList = [];
      }
      currentList.push(String(parseScalar(item)));
      continue;
    }

    const match = line.match(/^([^:]+):\s*(.*)$/);
    if (!match) {
      continue;
    }

    commitList();
    const key = match[1].trim();
    const value = match[2];

    if (value.trim() === "") {
      currentKey = key;
      currentList = [];
      continue;
    }

    if (value.trim().startsWith("[")) {
      const list = value
        .trim()
        .replace(/^\[|\]$/g, "")
        .split(",")
        .map((item) => item.trim())
        .filter((item) => item.length > 0)
        .map((item) => String(parseScalar(item)));
      data[key] = list;
      continue;
    }

    data[key] = parseScalar(value);
  }

  commitList();
  return data;
};

const parseToml = (raw: string): Record<string, unknown> => {
  const data: Record<string, unknown> = {};
  const lines = raw.split(/\r?\n/);

  for (const line of lines) {
    if (line.trim() === "" || line.trim().startsWith("#")) {
      continue;
    }
    const match = line.match(/^([^=]+)=\s*(.*)$/);
    if (!match) {
      continue;
    }
    const key = match[1].trim();
    const value = match[2].trim();
    if (value.startsWith("[")) {
      const list = value
        .replace(/^\[|\]$/g, "")
        .split(",")
        .map((item) => item.trim())
        .filter((item) => item.length > 0)
        .map((item) => String(parseScalar(item)));
      data[key] = list;
      continue;
    }
    data[key] = parseScalar(value);
  }

  return data;
};

export const parseFrontmatter = (content: string): ParsedFrontmatter => {
  const split = detectFrontmatter(content);
  if (!split) {
    return { data: {}, body: content, format: null, raw: null };
  }

  const data =
    split.format === "yaml" ? parseYaml(split.raw) : parseToml(split.raw);
  return {
    data,
    body: split.body,
    format: split.format,
    raw: split.raw,
  };
};
