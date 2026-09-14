import { describe, expect, it } from "vitest";
import {
  canonicalizeTags,
  commitTagInput,
  normalizeTags,
  removeTagAt,
  tagsEqual,
} from "./tags";

describe("tag normalization", () => {
  it("trims values, discards blanks, and preserves the first spelling", () => {
    expect(normalizeTags([" client ", "", "CLIENT", "visual pack", "  "])).toEqual([
      "client",
      "visual pack",
    ]);
  });

  it("provides a case-insensitive canonical comparison form", () => {
    expect(canonicalizeTags([" Client ", "Visual Pack", "client"])).toEqual([
      "client",
      "visual pack",
    ]);
    expect(tagsEqual([" Client ", "visual pack", "CLIENT"], ["client", "visual pack"])).toBe(true);
    expect(tagsEqual(["client", "visual pack"], ["visual pack", "client"])).toBe(false);
  });
});

describe("tag input commits", () => {
  it("commits a single Enter-delimited tag and clears the input", () => {
    expect(commitTagInput([], " favorite ", "enter")).toEqual({
      tags: ["favorite"],
      input: "",
      added: ["favorite"],
      duplicates: [],
    });
  });

  it("commits every Enter-delimited segment, including tags with spaces", () => {
    expect(commitTagInput([], "client, visual pack", "enter")).toEqual({
      tags: ["client", "visual pack"],
      input: "",
      added: ["client", "visual pack"],
      duplicates: [],
    });
  });

  it("commits complete comma segments and retains an unfinished trailing segment", () => {
    expect(commitTagInput([], "client, visual pack", "comma")).toEqual({
      tags: ["client"],
      input: " visual pack",
      added: ["client"],
      duplicates: [],
    });
    expect(commitTagInput(["client"], "client, favorite,", "comma")).toEqual({
      tags: ["client", "favorite"],
      input: "",
      added: ["favorite"],
      duplicates: ["client"],
    });
  });

  it("ignores blank values and case-insensitive duplicates", () => {
    expect(commitTagInput(["Client"], " , CLIENT, favorite ", "enter")).toEqual({
      tags: ["Client", "favorite"],
      input: "",
      added: ["favorite"],
      duplicates: ["CLIENT"],
    });
  });
});

describe("tag removal", () => {
  it("removes one normalized tag without changing neighboring tags", () => {
    expect(removeTagAt(["client", "visual pack", "favorite"], 1)).toEqual(["client", "favorite"]);
    expect(removeTagAt(["client"], 4)).toEqual(["client"]);
  });
});
