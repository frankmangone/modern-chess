// `<modern-chess-board>` as a JSX tag. The skin goes in as an attribute, not a property.
declare module "solid-js" {
  namespace JSX {
    interface IntrinsicElements {
      "modern-chess-board": JSX.HTMLAttributes<HTMLElement> & { "attr:skin"?: string };
    }
  }
}

export {};
