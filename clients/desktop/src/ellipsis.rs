//! Single-line text that ends in "…" instead of spilling into the next column.

use iced::advanced::graphics::text::Paragraph;
use iced::advanced::text::{Paragraph as _, Text};
use iced::widget::text::{LineHeight, Shaping, Wrapping};
use iced::widget::{Responsive, responsive, text};
use iced::{Alignment, Font, Length, Pixels, Size, alignment};

const ELLIPSIS: &str = "\u{2026}";

/// `content` on one line at `size`, cut short with an ellipsis when it is wider than the space
/// the parent gives it. Set the cell width on the returned widget.
pub fn ellipsized<'a, Message: 'a>(
    content: impl Into<String>,
    size: f32,
    align_x: Alignment,
) -> Responsive<'a, Message> {
    let content = content.into();
    responsive(move |bounds| {
        text(fit(&content, size, bounds.width))
            .size(size)
            .wrapping(Wrapping::None)
            .width(Length::Fill)
            .align_x(align_x)
            .into()
    })
    .height(Length::Shrink)
}

/// The longest prefix of `content` that, followed by an ellipsis, fits in `max_width`.
fn fit(content: &str, size: f32, max_width: f32) -> String {
    if width(content, size) <= max_width {
        return content.to_owned();
    }
    let ends: Vec<usize> = content
        .char_indices()
        .map(|(i, _)| i)
        .skip(1)
        .chain([content.len()])
        .collect();
    // Binary search for how many characters still fit before the ellipsis.
    let (mut low, mut high) = (0, ends.len());
    while low < high {
        let mid = (low + high).div_ceil(2);
        let candidate = format!("{}{ELLIPSIS}", content[..ends[mid - 1]].trim_end());
        if width(&candidate, size) <= max_width {
            low = mid;
        } else {
            high = mid - 1;
        }
    }
    match low {
        0 => ELLIPSIS.to_owned(),
        n => format!("{}{ELLIPSIS}", content[..ends[n - 1]].trim_end()),
    }
}

fn width(content: &str, size: f32) -> f32 {
    Paragraph::with_text(Text {
        content,
        bounds: Size::INFINITE,
        size: Pixels(size),
        line_height: LineHeight::default(),
        font: Font::DEFAULT,
        align_x: Alignment::Start.into(),
        align_y: alignment::Vertical::Top,
        shaping: Shaping::default(),
        wrapping: Wrapping::None,
    })
    .min_width()
}

#[cfg(test)]
mod tests {
    use super::*;

    const NAME: &str = "8b019c14-9952-4f96-8154-35181274837d.jpeg";

    #[test]
    fn keeps_text_that_fits() {
        assert_eq!(fit(NAME, 15.0, f32::INFINITY), NAME);
    }

    #[test]
    fn cuts_long_text_to_the_width() {
        let cut = fit(NAME, 15.0, 120.0);
        assert!(cut.ends_with(ELLIPSIS));
        assert!(NAME.starts_with(cut.trim_end_matches(ELLIPSIS)));
        assert!(width(&cut, 15.0) <= 120.0);
    }

    #[test]
    fn leaves_only_the_ellipsis_when_nothing_fits() {
        assert_eq!(fit(NAME, 15.0, 1.0), ELLIPSIS);
    }
}
