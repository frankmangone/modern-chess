# Piece pictures

Pictures that skins name with `{ "image": "/skins/<set>/<file>.svg" }`. Each set has its own folder.

| Folder | Used by | Source | Licence |
|---|---|---|---|
| `walnut-chess/` | Chess, Atomic, Chaturanga | `rhosgfx/` recoloured to the site's walnut palette: boxwood and dark chocolate, walnut outlines | CC0 1.0, as the original |
| `walnut-shogi/` | Shogi | `kahu-brown/` recoloured: pale boxwood, heavier walnut outline, brown ink, rust red for promoted pieces | CC BY 4.0, as the original. Credit (with the change noted) is shown on the Shogi page |
| `rhosgfx/` | Not used; source of `walnut-chess/` | RhosGFX, *Vector chess pieces*, taken from the [Lichess piece folder](https://github.com/lichess-org/lila/tree/master/public/piece/rhosgfx) | CC0 1.0 (no credit required) |
| `kahu-brown/` | Not used; source of `walnut-shogi/` | Ka-hu, [shogi-pieces](https://github.com/Ka-hu/shogi-pieces), the `kanji_brown` set (upright pieces, one kanji each) | CC BY 4.0. Credit is shown on the Shogi page |
| `modern-chess/` | Checkers, Ataxx | Drawn for this project | Project-owned |

On the web app's boards every drawn piece gets a hard shadow from `styles.css`, so none of these files draw their own.
