#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativePalette {
    pub id: &'static str,
    pub bg: &'static str,
    pub fg: &'static str,
    pub fg_3: &'static str,
    pub accent: &'static str,
    pub warn: &'static str,
    pub danger: &'static str,
}

#[allow(dead_code)]
pub mod themes {
    use super::NativePalette;

    pub const GRAFITE: NativePalette = NativePalette {
        id: "grafite",
        bg: "#141412",
        fg: "#F0EDE5",
        fg_3: "#6A6760",
        accent: "#00DA7E",
        warn: "#F7B83D",
        danger: "#FF645F",
    };

    pub const BLUEPRINT: NativePalette = NativePalette {
        id: "blueprint",
        bg: "#0D1520",
        fg: "#E4EDF7",
        fg_3: "#5E7491",
        accent: "#55C4FE",
        warn: "#F4C352",
        danger: "#F66D67",
    };

    pub const FOSFORO: NativePalette = NativePalette {
        id: "fosforo",
        bg: "#13100A",
        fg: "#F4E7CB",
        fg_3: "#776849",
        accent: "#FCB02B",
        warn: "#F0CF4C",
        danger: "#F36451",
    };

    pub const BANCADA: NativePalette = NativePalette {
        id: "bancada",
        bg: "#0F1512",
        fg: "#E3EFE7",
        fg_3: "#62786B",
        accent: "#55E982",
        warn: "#F4C352",
        danger: "#FA6863",
    };

    pub const PAPEL: NativePalette = NativePalette {
        id: "papel",
        bg: "#F5F2EC",
        fg: "#17160E",
        fg_3: "#7A776F",
        accent: "#007E46",
        warn: "#A96B00",
        danger: "#CC2827",
    };

    pub const CONTRASTE: NativePalette = NativePalette {
        id: "contraste",
        bg: "#000000",
        fg: "#FFFFFF",
        fg_3: "#B3B3B3",
        accent: "#C0F447",
        warn: "#FBD530",
        danger: "#FF716B",
    };

    pub const ALL: [NativePalette; 6] = [GRAFITE, BLUEPRINT, FOSFORO, BANCADA, PAPEL, CONTRASTE];
}

pub const DEFAULT_THEME: &str = "grafite";
