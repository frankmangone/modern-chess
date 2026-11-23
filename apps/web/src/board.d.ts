// `<modern-chess-board>` as a JSX tag. The skin goes in as an attribute, not a property.
declare module "solid-js" {
  namespace JSX {
    interface IntrinsicElements {
      "modern-chess-board": JSX.HTMLAttributes<HTMLElement> & {
        "attr:skin"?: string;
        "attr:remote"?: boolean;
        "prop:view"?: unknown;
        "prop:interactive"?: boolean;
        "prop:flipped"?: boolean;
        "prop:externalPrompts"?: boolean;
        "on:choose"?: (event: Event) => void;
        "on:cancel"?: (event: Event) => void;
      };
    }
  }
}

export {};
