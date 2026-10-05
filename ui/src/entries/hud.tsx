import { HudWindow } from "../features/hud/hud-window";

import { boot } from "./boot";

document.documentElement.classList.add("transparent-window");
boot(<HudWindow />);
