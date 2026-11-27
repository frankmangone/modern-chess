/** The skin's text read as a skin: nothing, a skin, or why it is not one. */
export function readSkin(text: string): { skin: Record<string, unknown> | null; problem: string | null } {
  if (text.trim() === "") return { skin: null, problem: null };
  try {
    const skin: unknown = JSON.parse(text);
    if (typeof skin === "object" && skin !== null && !Array.isArray(skin)) {
      return { skin: skin as Record<string, unknown>, problem: null };
    }
    return { skin: null, problem: "A look is an object: { … }." };
  } catch (error) {
    return { skin: null, problem: error instanceof Error ? error.message : "That is not JSON." };
  }
}
