export type SSGId = "eleventy" | "hugo" | "jekyll" | "gatsby" | "astro";

export type FrontmatterFormat = "yaml" | "toml";

export type SchemaFieldType =
  | "string"
  | "string_list"
  | "date"
  | "boolean"
  | "number"
  | "enum";

export type ValidationStatus = "ok" | "warning" | "error";

export type ValidationIssue = {
  status: ValidationStatus;
  message: string;
  field?: string;
  code?: string;
};

export type FieldRuleType =
  | "minLength"
  | "maxLength"
  | "pattern"
  | "minItems"
  | "maxItems"
  | "uniqueItems"
  | "itemMinLength"
  | "itemMaxLength"
  | "dateNotFuture"
  | "enumValues";

export type FieldRule = {
  type: FieldRuleType;
  value?: number | string | boolean | string[];
  severity: ValidationStatus;
  message: string;
};

export type FrontmatterSchemaField = {
  name: string;
  label: string;
  type: SchemaFieldType;
  required?: boolean;
  defaultValue?: unknown;
  description?: string;
  rules?: FieldRule[];
};

export type FrontmatterSchema = {
  id: string;
  fields: FrontmatterSchemaField[];
};
