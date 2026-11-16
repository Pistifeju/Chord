// Rules for the Simulator view's Appearance menu: how a Dynamic Type size reads, and the language and region codes by name.

/** "extra-large" reads "Extra Large". */
export const sizeLabel = (category: string) =>
  category
    .split("-")
    .map((word) => word.charAt(0).toUpperCase() + word.slice(1))
    .join(" ");

/** The codes with the name `nameOf` gives each, in name order; a code without a name keeps the code. */
export function namedCodes(codes: string[], nameOf: (code: string) => string | undefined): { code: string; name: string }[] {
  return codes.map((code) => ({ code, name: nameOf(code) ?? code })).sort((a, b) => a.name.localeCompare(b.name));
}
