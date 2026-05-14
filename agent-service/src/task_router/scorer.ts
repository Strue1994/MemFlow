export function tokenize(text: string): string[] {
  const matches = text.toLowerCase().match(/[a-z0-9_]+|[\u4e00-\u9fff]+/g);
  return matches ?? [];
}

export function overlapScore(left: string, right: string): number {
  const leftTokens = new Set(tokenize(left));
  const rightTokens = new Set(tokenize(right));

  if (leftTokens.size === 0 || rightTokens.size === 0) {
    return 0;
  }

  let overlapCount = 0;
  for (const token of leftTokens) {
    if (rightTokens.has(token)) {
      overlapCount += 1;
    }
  }

  const unionSize = new Set([...leftTokens, ...rightTokens]).size;
  return unionSize === 0 ? 0 : overlapCount / unionSize;
}
