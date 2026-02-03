import { describe, expect, it } from "vitest";
import type { FrontmatterSchema } from "../types";
import { validateData } from "./engine";

const schema: FrontmatterSchema = {
  id: "test",
  fields: [
    { name: "title", label: "Title", type: "string", required: true },
    {
      name: "tags",
      label: "Tags",
      type: "string_list",
      rules: [{ type: "minItems", severity: "warning", value: 1, message: "Add at least one tag" }],
    },
    {
      name: "published",
      label: "Published",
      type: "date",
      rules: [{ type: "dateNotFuture", severity: "error", message: "Date cannot be in the future" }],
    },
  ],
};

describe("validateData", () => {
  it("flags missing required fields", () => {
    const issues = validateData(schema, {});
    expect(issues.some((issue) => issue.code === "required" && issue.field === "title")).toBe(true);
  });

  it("accepts valid data and warns on optional rule", () => {
    const issues = validateData(schema, {
      title: "Ok",
      tags: ["one"],
      published: "2020-01-01",
    });
    expect(issues.length).toBe(0);
  });

  it("rejects future dates", () => {
    const future = new Date(Date.now() + 24 * 60 * 60 * 1000).toISOString();
    const issues = validateData(schema, { title: "Ok", published: future });
    const dateIssue = issues.find((issue) => issue.field === "published");
    expect(dateIssue?.status).toBe("error");
  });
});
