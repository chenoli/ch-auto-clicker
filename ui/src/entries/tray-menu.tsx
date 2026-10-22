import { TrayMenuWindow } from "../features/tray/tray-menu-window";

import { boot } from "./boot";

document.documentElement.classList.add("transparent-window");
boot(<TrayMenuWindow />);
