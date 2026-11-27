// How Clerk's sign-in and sign-up screens look: the site's Walnut, so the modal reads as part of
// the page and not as a visitor from another product. The values are the ones in `styles.css`
// (`:root`): Clerk takes literal colours here, not our custom properties, so change them together.
// The outlines and shadows are plain CSS in `styles.css`, which does use the properties.

const PAPER = "#f3e5cb";
const PAPER_2 = "#e9d6b4";
const INK = "#24130a";
const INK_SOFT = "#5e3f29";
const LINE = "#1c0e06";
const SHADOW = "#140903";
const HONEY = "#f2c66b";
const DEEP_EMBER = "#7a2606";

const HEADING = '"Bricolage Grotesque", system-ui, sans-serif';
const BODY = '"Literata", Georgia, serif';

export const clerkAppearance = {
  // Clerk's own styles go in a cascade layer, so the rules in `styles.css` (under "CLERK") win
  // without `!important`. They give the outlines and hard shadows, which Clerk's variables cannot.
  cssLayerName: "clerk",
  variables: {
    colorPrimary: HONEY,
    colorPrimaryForeground: LINE,
    colorBackground: PAPER,
    colorForeground: INK,
    colorMutedForeground: INK_SOFT,
    colorMuted: PAPER_2,
    colorInput: PAPER_2,
    colorInputForeground: INK,
    colorNeutral: INK,
    colorBorder: LINE,
    colorRing: HONEY,
    colorDanger: DEEP_EMBER,
    colorShadow: SHADOW,
    colorModalBackdrop: "rgba(20, 9, 3, 0.72)",
    fontFamily: BODY,
    fontFamilyButtons: HEADING,
    borderRadius: "12px",
  },
};
