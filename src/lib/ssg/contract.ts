import type {
  FrontmatterFormat,
  FrontmatterSchema,
  SSGId,
  ValidationIssue,
} from "../types";

export type FrontmatterData = Record<string, unknown>;

export type SSGPlugin = {
  id: SSGId;
  label: string;
  formats: FrontmatterFormat[];
  getSchema: () => FrontmatterSchema;
  getDefaults: () => FrontmatterData;
  validate: (data: FrontmatterData) => ValidationIssue[];
  serialize: (data: FrontmatterData, format: FrontmatterFormat) => string;
};
