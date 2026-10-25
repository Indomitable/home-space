use iced::{
    Background, Border, Color, Shadow, Theme, Vector,
    border::Radius,
    widget::{button, container},
};

const RADIUS: f32 = 6.0;
const PANEL_RADIUS: f32 = 10.0;
const BACKDROP_ALPHA: f32 = 0.55;
const SELECTED_ALPHA: f32 = 0.22;
const SELECTED_HOVER_ALPHA: f32 = 0.3;
const SHADOW_BLUR: f32 = 24.0;
const SHADOW_OFFSET: f32 = 8.0;
const SHADOW_ALPHA: f32 = 0.35;

/// A grid row: transparent, tinted on hover, accent-tinted when selected.
pub fn row(selected: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |theme, status| {
        let palette = theme.extended_palette();
        let accent = palette.primary.base.color;
        let background = match (selected, status) {
            (true, button::Status::Hovered | button::Status::Pressed) => {
                Some(accent.scale_alpha(SELECTED_HOVER_ALPHA))
            }
            (true, _) => Some(accent.scale_alpha(SELECTED_ALPHA)),
            (false, button::Status::Hovered) => Some(palette.background.weak.color),
            (false, button::Status::Pressed) => Some(palette.background.strong.color),
            (false, _) => None,
        };
        button::Style {
            background: background.map(Background::Color),
            text_color: palette.background.base.text,
            border: Border::default().rounded(RADIUS),
            ..button::Style::default()
        }
    }
}

/// The sticky header above the grid.
pub fn header(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();
    container::Style {
        background: Some(palette.background.weak.color.into()),
        text_color: Some(palette.background.strongest.color),
        border: Border::default().rounded(RADIUS),
        ..container::Style::default()
    }
}

/// A flat toolbar or menu button that highlights on hover.
pub fn tool(theme: &Theme, status: button::Status) -> button::Style {
    let palette = theme.extended_palette();
    let background = match status {
        button::Status::Hovered => Some(palette.background.weak.color),
        button::Status::Pressed => Some(palette.background.strong.color),
        button::Status::Active | button::Status::Disabled => None,
    };
    let text_color = match status {
        button::Status::Disabled => palette.background.base.text.scale_alpha(0.4),
        _ => palette.background.base.text,
    };
    button::Style {
        background: background.map(Background::Color),
        text_color,
        border: Border::default().rounded(RADIUS),
        ..button::Style::default()
    }
}

/// Like [`tool`], with danger-coloured text.
pub fn danger_tool(theme: &Theme, status: button::Status) -> button::Style {
    let danger = theme.extended_palette().danger.base.color;
    let base = tool(theme, status);
    match status {
        button::Status::Disabled => base,
        _ => button::Style {
            text_color: danger,
            ..base
        },
    }
}

/// The dimmed layer behind a dialog.
pub fn backdrop(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Color::BLACK.scale_alpha(BACKDROP_ALPHA).into()),
        ..container::Style::default()
    }
}

/// A raised surface: dialogs, the context menu and the transfer panel.
pub fn panel(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();
    container::Style {
        background: Some(palette.background.base.color.into()),
        text_color: Some(palette.background.base.text),
        border: Border {
            color: palette.background.strong.color,
            width: 1.0,
            radius: Radius::from(PANEL_RADIUS),
        },
        shadow: Shadow {
            color: Color::BLACK.scale_alpha(SHADOW_ALPHA),
            offset: Vector::new(0.0, SHADOW_OFFSET),
            blur_radius: SHADOW_BLUR,
        },
        ..container::Style::default()
    }
}

/// A single-line error message.
pub fn error(theme: &Theme) -> container::Style {
    let danger = theme.extended_palette().danger.base.color;
    container::Style {
        text_color: Some(danger),
        ..container::Style::default()
    }
}
