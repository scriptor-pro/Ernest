import type { FrontmatterFormat } from "../types";

const isPlainString = (value: string): boolean =>
  /^[A-Za-z0-9_\-\/]+$/.test(value);

const escapeString = (value: string): string =>
  value.replace(/\\/g, "\\\\").replace(/\"/g, '\\"');

const stringifyString = (value: string): string => {
  if (value === "") {
    return '""';
  }
  if (isPlainString(value)) {
    return value;
  }
  return `"${escapeString(value)}"`;
};

const stringifyScalar = (value: unknown): string => {
  if (value instanceof Date) {
    return value.toISOString();
  }
  if (typeof value === "string") {
    return stringifyString(value);
  }
  if (typeof value === "number") {
    return Number.isNaN(value) ? "0" : String(value);
  }
  if (typeof value === "boolean") {
    return value ? "true" : "false";
  }
  if (value === null || value === undefined) {
    return "";
  }
  return stringifyString(String(value));
};

const serializeYaml = (data: Record<string, unknown>): string => {
  const lines: string[] = ["---"];
  Object.keys(data).forEach((key) => {
    const value = data[key];
    if (Array.isArray(value)) {
      lines.push(`${key}:`);
      value.forEach((item) => {
        lines.push(`- ${stringifyScalar(item)}`);
      });
      return;
    }
    lines.push(`${key}: ${stringifyScalar(value)}`);
  });
  lines.push("---");
  return lines.join("\n");
};

const serializeToml = (data: Record<string, unknown>): string => {
  const lines: string[] = ["+++"];
  Object.keys(data).forEach((key) => {
    const value = data[key];
    if (Array.isArray(value)) {
      const items = value.map((item) => stringifyScalar(item)).join(", ");
      lines.push(`${key} = [${items}]`);
      return;
    }
    lines.push(`${key} = ${stringifyScalar(value)}`);
  });
  lines.push("+++");
  return lines.join("\n");
};

export const serializeFrontmatter = (
  data: Record<string, unknown>,
  format: FrontmatterFormat,
): string => (format === "yaml" ? serializeYaml(data) : serializeToml(data));
