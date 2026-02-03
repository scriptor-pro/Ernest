import { describe, expect, it } from "vitest";
import { serializeFrontmatter } from "./serialize";

describe("serializeFrontmatter", () => {
  it("serializes YAML with arrays and scalars", () => {
    const output = serializeFrontmatter({ title: "Doc", tags: ["one", "two"], draft: true }, "yaml");
    expect(output.split("\n")).toEqual([
      "---",
      "title: Doc",
      "tags:",
      "- one",
      "- two",
      "draft: true",
      "---",
    ]);
  });

  it("serializes TOML with arrays", () => {
    const output = serializeFrontmatter({ title: "Doc", tags: ["one", "two"], views: 3 }, "toml");
    expect(output.split("\n")).toEqual([
      "+++",
      "title = Doc",
      "tags = [one, two]",
      "views = 3",
      "+++",
    ]);
  });
});
