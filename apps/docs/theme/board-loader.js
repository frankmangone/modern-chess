// A classic script so mdBook can include it. It loads the board as an ES module,
// relative to this file. If that fails, the chapter text stays readable.
import("./modern-chess-board.js").catch((error) => {
  console.error("The playable boards could not be loaded:", error);
});
