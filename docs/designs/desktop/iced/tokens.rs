//! Reference tokens for the desktop redesign (docs/designs/desktop). Not compiled —
//! copy into clients/desktop/src/theme.rs when implementing.
//!
//! The values are the home-space design system's (the web client's `styles/tokens.css`),
//! plus one new token, `sidebar`. Style functions should read these instead of the
//! extended palette iced generates, so light and dark match the web client exactly.

use iced::{Color, Theme, color, theme::Palette};

#[derive(Debug, Clone, Copy)]
pub struct Tokens {
    pub bg: Color,
    pub surface: Color,
    pub surface_hover: Color,
    pub sidebar: Color,
    pub text: Color,
    pub muted: Color,
    pub border: Color,
    pub border_strong: Color,
    pub accent: Color,
    pub accent_hover: Color,
    pub on_accent: Color,
    /// accent at 12%: selected rows, active sidebar item, drop overlay.
    pub accent_soft: Color,
    pub danger: Color,
    /// danger at 12%: error alerts, danger hovers.
    pub danger_soft: Color,
    pub success: Color,
    pub backdrop: Color,
    pub shadow: Color,
}

pub const LIGHT: Tokens = Tokens {
    bg: color!(0xf4f5f7),
    surface: color!(0xffffff),
    surface_hover: color!(0xf2f4f7),
    sidebar: color!(0xeceef2),
    text: color!(0x14171c),
    muted: color!(0x5c6573),
    border: color!(0xe2e5ea),
    border_strong: color!(0xcdd2da),
    accent: color!(0x2563eb),
    accent_hover: color!(0x1d4ed8),
    on_accent: color!(0xffffff),
    accent_soft: color!(0x2563eb, 0.12),
    danger: color!(0xc0362c),
    danger_soft: color!(0xc0362c, 0.12),
    success: color!(0x15803d),
    backdrop: color!(0x080a0e, 0.55),
    shadow: color!(0x101828, 0.08),
};

pub const DARK: Tokens = Tokens {
    bg: color!(0x0e1014),
    surface: color!(0x171a20),
    surface_hover: color!(0x1e222a),
    sidebar: color!(0x13161b),
    text: color!(0xe7e9ec),
    muted: color!(0x98a1b0),
    border: color!(0x262b34),
    border_strong: color!(0x353b46),
    accent: color!(0x6d9eff),
    accent_hover: color!(0x8ab1ff),
    on_accent: color!(0x0a1220),
    accent_soft: color!(0x6d9eff, 0.12),
    danger: color!(0xff8a80),
    danger_soft: color!(0xff8a80, 0.12),
    success: color!(0x5fd39a),
    backdrop: color!(0x080a0e, 0.55),
    shadow: color!(0x000000, 0.45),
};

/// Radii: controls and menu items, in-page boxes and menus, dialogs and floating panels.
pub const RADIUS_SM: f32 = 6.0;
pub const RADIUS_MD: f32 = 10.0;
pub const RADIUS_LG: f32 = 16.0;

/// The iced theme for a mode. Built-in widgets (text inputs, scrollables, checkboxes)
/// take their colours from this palette; the app's own style functions use [`Tokens`].
pub fn theme(dark: bool) -> Theme {
    let t = tokens(dark);
    let name = if dark { "Home Space Dark" } else { "Home Space Light" };
    Theme::custom(
        name.to_owned(),
        Palette {
            background: t.surface,
            text: t.text,
            primary: t.accent,
            success: t.success,
            // No warning colour in the design system yet; iced needs one.
            warning: if dark { color!(0xfbbf24) } else { color!(0xb45309) },
            danger: t.danger,
        },
    )
}

pub fn tokens(dark: bool) -> Tokens {
    if dark { DARK } else { LIGHT }
}
