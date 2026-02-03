import { describe, expect, it } from "vitest";
import { parseFrontmatter } from "./parse";

describe("parseFrontmatter", () => {
  it("returns body only when no frontmatter is present", () => {
    const result = parseFrontmatter("# Title\ncontent");
    expect(result.format).toBeNull();
    expect(result.raw).toBeNull();
    expect(result.data).toEqual({});
    expect(result.body).toContain("content");
  });

  it("parses YAML frontmatter and body", () => {
    const input = `---\ntitle: Post\ntags: [one, two]\n---\n\nBody text`;
    const result = parseFrontmatter(input);
    expect(result.format).toBe("yaml");
    expect(result.raw).toContain("title: Post");
    expect(result.data).toEqual({ title: "Post", tags: ["one", "two"] });
    expect(result.body.trim()).toBe("Body text");
  });

  it("parses TOML frontmatter", () => {
    const input = `+++\ntitle = "Doc"\ntags = ["a", "b"]\n+++\nHello`;
    const result = parseFrontmatter(input);
    expect(result.format).toBe("toml");
    expect(result.data).toEqual({ title: "Doc", tags: ["a", "b"] });
    expect(result.body.trim()).toBe("Hello");
  });
});
