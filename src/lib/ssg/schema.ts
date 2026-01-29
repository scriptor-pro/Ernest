import type { FrontmatterSchema } from "../types";

export const minimalSchema: FrontmatterSchema = {
  id: "minimal",
  fields: [
    {
      name: "title",
      label: "Title",
      type: "string",
      required: true,
      rules: [
        {
          type: "minLength",
          value: 3,
          severity: "error",
          message: "Title must be at least 3 characters.",
        },
        {
          type: "maxLength",
          value: 120,
          severity: "error",
          message: "Title must be 120 characters or fewer.",
        },
      ],
    },
    {
      name: "description",
      label: "Description",
      type: "string",
      rules: [
        {
          type: "maxLength",
          value: 240,
          severity: "warning",
          message: "Description should be 240 characters or fewer.",
        },
      ],
    },
    {
      name: "date",
      label: "Date",
      type: "date",
      rules: [
        {
          type: "dateNotFuture",
          severity: "warning",
          message: "Date is in the future.",
        },
      ],
    },
    {
      name: "tags",
      label: "Tags",
      type: "string_list",
      rules: [
        {
          type: "uniqueItems",
          severity: "warning",
          message: "Tags should be unique.",
        },
        {
          type: "maxItems",
          value: 20,
          severity: "error",
          message: "Tags must contain 20 items or fewer.",
        },
        {
          type: "itemMinLength",
          value: 2,
          severity: "error",
          message: "Each tag must be at least 2 characters.",
        },
        {
          type: "itemMaxLength",
          value: 32,
          severity: "error",
          message: "Each tag must be 32 characters or fewer.",
        },
      ],
    },
    {
      name: "categories",
      label: "Categories",
      type: "string_list",
      rules: [
        {
          type: "uniqueItems",
          severity: "warning",
          message: "Categories should be unique.",
        },
        {
          type: "maxItems",
          value: 10,
          severity: "error",
          message: "Categories must contain 10 items or fewer.",
        },
        {
          type: "itemMinLength",
          value: 2,
          severity: "error",
          message: "Each category must be at least 2 characters.",
        },
        {
          type: "itemMaxLength",
          value: 32,
          severity: "error",
          message: "Each category must be 32 characters or fewer.",
        },
      ],
    },
    {
      name: "draft",
      label: "Draft",
      type: "boolean",
    },
    {
      name: "slug",
      label: "Slug",
      type: "string",
      description:
        "URL-friendly identifier, usually lowercase with hyphens (ex: my-first-post).",
      rules: [
        {
          type: "pattern",
          value: "^[a-z0-9]+(?:-[a-z0-9]+)*$",
          severity: "error",
          message: "Slug must use lowercase letters, numbers, and hyphens.",
        },
      ],
    },
  ],
};

export const eleventySchema: FrontmatterSchema = {
  id: "eleventy",
  fields: [
    ...minimalSchema.fields,
    {
      name: "layout",
      label: "Layout",
      type: "string",
      rules: [
        {
          type: "maxLength",
          value: 120,
          severity: "warning",
          message: "Layout should be 120 characters or fewer.",
        },
      ],
    },
    {
      name: "permalink",
      label: "Permalink",
      type: "string",
      description:
        "Full output path, often starting with a slash (ex: /blog/my-first-post/).",
      rules: [
        {
          type: "pattern",
          value: "^/.*",
          severity: "warning",
          message: "Permalink should start with a slash.",
        },
      ],
    },
    {
      name: "eleventyExcludeFromCollections",
      label: "Exclude from collections",
      type: "boolean",
    },
  ],
};
