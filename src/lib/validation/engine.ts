import type {
  FrontmatterSchema,
  ValidationIssue,
  FieldRule,
  ValidationStatus,
  SchemaFieldType,
} from "../types";

const isEmptyValue = (value: unknown): boolean => {
  if (value === undefined || value === null) {
    return true;
  }
  if (typeof value === "string") {
    return value.trim() === "";
  }
  return false;
};

const toStringArray = (value: unknown): string[] | null => {
  if (Array.isArray(value)) {
    return value.map((item) => String(item));
  }
  if (typeof value === "string") {
    return value
      .split(",")
      .map((item) => item.trim())
      .filter((item) => item.length > 0);
  }
  return null;
};

const parseDate = (value: unknown): Date | null => {
  if (value instanceof Date) {
    return Number.isNaN(value.getTime()) ? null : value;
  }
  if (typeof value === "string" && value.trim().length > 0) {
    const parsed = new Date(value);
    return Number.isNaN(parsed.getTime()) ? null : parsed;
  }
  return null;
};

const compareStatus = (a: ValidationStatus, b: ValidationStatus): number => {
  const order: Record<ValidationStatus, number> = {
    ok: 0,
    warning: 1,
    error: 2,
  };
  return order[b] - order[a];
};

const buildIssue = (rule: FieldRule, fieldName: string): ValidationIssue => ({
  status: rule.severity,
  message: rule.message,
  field: fieldName,
});

const applyRule = (
  rule: FieldRule,
  value: unknown,
  fieldType: SchemaFieldType,
  fieldName: string,
): ValidationIssue | null => {
  switch (rule.type) {
    case "minLength":
      if (typeof value === "string" && value.length < Number(rule.value)) {
        return buildIssue(rule, fieldName);
      }
      return null;
    case "maxLength":
      if (typeof value === "string" && value.length > Number(rule.value)) {
        return buildIssue(rule, fieldName);
      }
      return null;
    case "pattern":
      if (typeof value === "string") {
        const regex = new RegExp(String(rule.value));
        if (!regex.test(value)) {
          return buildIssue(rule, fieldName);
        }
      }
      return null;
    case "minItems": {
      const list = toStringArray(value);
      if (list && list.length < Number(rule.value)) {
        return buildIssue(rule, fieldName);
      }
      return null;
    }
    case "maxItems": {
      const list = toStringArray(value);
      if (list && list.length > Number(rule.value)) {
        return buildIssue(rule, fieldName);
      }
      return null;
    }
    case "uniqueItems": {
      const list = toStringArray(value);
      if (list) {
        const uniqueCount = new Set(list.map((item) => item.trim())).size;
        if (uniqueCount !== list.length) {
          return buildIssue(rule, fieldName);
        }
      }
      return null;
    }
    case "itemMinLength": {
      const list = toStringArray(value);
      if (list && list.some((item) => item.length < Number(rule.value))) {
        return buildIssue(rule, fieldName);
      }
      return null;
    }
    case "itemMaxLength": {
      const list = toStringArray(value);
      if (list && list.some((item) => item.length > Number(rule.value))) {
        return buildIssue(rule, fieldName);
      }
      return null;
    }
    case "dateNotFuture": {
      const date = parseDate(value);
      if (date) {
        const now = new Date();
        if (date.getTime() > now.getTime()) {
          return buildIssue(rule, fieldName);
        }
      }
      return null;
    }
    case "enumValues": {
      const values = Array.isArray(rule.value) ? rule.value : [];
      if (typeof value === "string" && values.length > 0) {
        if (!values.includes(value)) {
          return buildIssue(rule, fieldName);
        }
      }
      return null;
    }
    default:
      return null;
  }
};

const validateRequired = (
  fieldName: string,
  value: unknown,
): ValidationIssue | null => {
  if (isEmptyValue(value)) {
    return {
      status: "error",
      message: "This field is required.",
      field: fieldName,
      code: "required",
    };
  }
  return null;
};

const validateType = (
  fieldName: string,
  value: unknown,
  fieldType: SchemaFieldType,
): ValidationIssue | null => {
  if (isEmptyValue(value)) {
    return null;
  }
  switch (fieldType) {
    case "string":
      return typeof value === "string"
        ? null
        : {
            status: "error",
            message: "Expected a text value.",
            field: fieldName,
            code: "type",
          };
    case "string_list":
      return toStringArray(value) !== null
        ? null
        : {
            status: "error",
            message: "Expected a list of text values.",
            field: fieldName,
            code: "type",
          };
    case "date":
      return parseDate(value)
        ? null
        : {
            status: "error",
            message: "Expected a valid date.",
            field: fieldName,
            code: "type",
          };
    case "boolean":
      return typeof value === "boolean"
        ? null
        : {
            status: "error",
            message: "Expected a boolean value.",
            field: fieldName,
            code: "type",
          };
    case "number":
      return typeof value === "number" && !Number.isNaN(value)
        ? null
        : {
            status: "error",
            message: "Expected a numeric value.",
            field: fieldName,
            code: "type",
          };
    case "enum":
      return typeof value === "string"
        ? null
        : {
            status: "error",
            message: "Expected a value from the list.",
            field: fieldName,
            code: "type",
          };
    default:
      return null;
  }
};

export const validateData = (
  schema: FrontmatterSchema,
  data: Record<string, unknown>,
): ValidationIssue[] => {
  const issues: ValidationIssue[] = [];

  schema.fields.forEach((field) => {
    const value = data[field.name];

    if (field.required) {
      const requiredIssue = validateRequired(field.name, value);
      if (requiredIssue) {
        issues.push(requiredIssue);
        return;
      }
    }

    const typeIssue = validateType(field.name, value, field.type);
    if (typeIssue) {
      issues.push(typeIssue);
      return;
    }

    field.rules?.forEach((rule) => {
      const issue = applyRule(rule, value, field.type, field.name);
      if (issue) {
        issues.push(issue);
      }
    });
  });

  return issues.sort((a, b) => compareStatus(a.status, b.status));
};
