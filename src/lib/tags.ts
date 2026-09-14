export type TagCommitTrigger = "comma" | "enter";

export type TagCommitResult = {
  tags: string[];
  input: string;
  added: string[];
  duplicates: string[];
};

function tagKey(tag: string): string {
  return tag.toLowerCase();
}

/**
 * Trims tags, removes blanks, and keeps the first spelling of each
 * case-insensitive value.
 */
export function normalizeTags(values: readonly string[]): string[] {
  const seen = new Set<string>();
  const normalized: string[] = [];

  for (const value of values) {
    const tag = value.trim();
    if (!tag) continue;

    const key = tagKey(tag);
    if (seen.has(key)) continue;

    seen.add(key);
    normalized.push(tag);
  }

  return normalized;
}

/**
 * Returns the stable comparison form used for dirty-state checks.
 */
export function canonicalizeTags(values: readonly string[]): string[] {
  return normalizeTags(values).map(tagKey);
}

export function tagsEqual(left: readonly string[], right: readonly string[]): boolean {
  const canonicalLeft = canonicalizeTags(left);
  const canonicalRight = canonicalizeTags(right);

  return (
    canonicalLeft.length === canonicalRight.length &&
    canonicalLeft.every((tag, index) => tag === canonicalRight[index])
  );
}

/**
 * Commits complete comma-delimited segments and returns the unfinished
 * trailing segment to the editor. Enter commits the whole input.
 */
export function commitTagInput(
  existingTags: readonly string[],
  input: string,
  trigger: TagCommitTrigger,
): TagCommitResult {
  const segments = input.split(",");
  let completeSegments: string[];
  let remainingInput = "";

  if (trigger === "enter") {
    completeSegments = segments;
  } else if (segments.length === 1 || input.endsWith(",")) {
    completeSegments = input.endsWith(",") ? segments.slice(0, -1) : segments;
  } else {
    completeSegments = segments.slice(0, -1);
    remainingInput = segments[segments.length - 1] ?? "";
  }

  const tags = normalizeTags(existingTags);
  const seen = new Set(tags.map(tagKey));
  const added: string[] = [];
  const duplicates: string[] = [];

  for (const value of completeSegments) {
    const tag = value.trim();
    if (!tag) continue;

    const key = tagKey(tag);
    if (seen.has(key)) {
      duplicates.push(tag);
      continue;
    }

    seen.add(key);
    added.push(tag);
    tags.push(tag);
  }

  return { tags, input: remainingInput, added, duplicates };
}

export function removeTagAt(values: readonly string[], index: number): string[] {
  return normalizeTags(values).filter((_, valueIndex) => valueIndex !== index);
}
