import { HudWindow } from "../features/hud/hud-window";

import { boot } from "./boot";

// The HUD window is transparent: only the pill paints.
document.documentElement.classList.add("transparent-window");
boot(<HudWindow />);
