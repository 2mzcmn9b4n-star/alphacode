//! UI Polish Layer — visual refinement and modern design tokens.
//!
//! This module provides enhanced visual styling for the TUI, including:
//! - Modern color palette with better contrast
//! - Refined spacing and typography
//! - Smooth animations and transitions
//! - Enhanced component styling
//! - Accessibility improvements

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Padding},
};

/// Modern color palette with WCAG AA contrast compliance.
pub mod palette {
    use ratatui::style::Color;

    // Brand colors — vibrant but not overwhelming
    pub const BRAND_ACCENT: Color = Color::Rgb(0, 200, 255);      // Electric cyan
    pub const BRAND_ACCENT_DIM: Color = Color::Rgb(0, 140, 200);  // Muted cyan
    pub const BRAND_ACCENT_GLOW: Color = Color::Rgb(0, 255, 200);  // Neon cyan

    // AI/Assistant colors
    pub const AI_PRIMARY: Color = Color::Rgb(130, 170, 255);      // Soft blue
    pub const AI_SECONDARY: Color = Color::Rgb(100, 140, 220);    // Muted blue
    pub const AI_TEXT: Color = Color::Rgb(220, 230, 255);         // Light blue-white

    // User colors
    pub const USER_PRIMARY: Color = Color::Rgb(255, 200, 100);    // Warm amber
    pub const USER_SECONDARY: Color = Color::Rgb(220, 170, 80);   // Muted amber
    pub const USER_TEXT: Color = Color::Rgb(255, 240, 220);       // Light amber-white

    // Status colors
    pub const SUCCESS: Color = Color::Rgb(80, 220, 120);          // Emerald green
    pub const WARNING: Color = Color::Rgb(255, 180, 60);          // Warm amber
    pub const ERROR: Color = Color::Rgb(255, 100, 100);           // Soft red
    pub const INFO: Color = Color::Rgb(100, 180, 255);            // Sky blue

    // Neutral colors
    pub const TEXT_PRIMARY: Color = Color::Rgb(240, 240, 245);    // Near white
    pub const TEXT_SECONDARY: Color = Color::Rgb(180, 180, 190);  // Light gray
    pub const TEXT_MUTED: Color = Color::Rgb(120, 120, 130);      // Medium gray
    pub const TEXT_DIM: Color = Color::Rgb(80, 80, 90);           // Dark gray

    // Background colors
    pub const BG_PRIMARY: Color = Color::Rgb(15, 15, 20);        // Near black
    pub const BG_SECONDARY: Color = Color::Rgb(25, 25, 35);       // Dark gray
    pub const BG_TERTIARY: Color = Color::Rgb(35, 35, 50);        // Medium dark
    pub const BG_ELEVATED: Color = Color::Rgb(45, 45, 65);        // Elevated surface

    // Border colors
    pub const BORDER_PRIMARY: Color = Color::Rgb(60, 60, 80);     // Visible border
    pub const BORDER_SECONDARY: Color = Color::Rgb(40, 40, 55);   // Subtle border
    pub const BORDER_ACCENT: Color = Color::Rgb(0, 180, 220);     // Accent border

    // Tool status colors
    pub const TOOL_RUNNING: Color = Color::Rgb(255, 200, 80);     // Amber
    pub const TOOL_SUCCESS: Color = Color::Rgb(80, 220, 120);     // Green
    pub const TOOL_ERROR: Color = Color::Rgb(255, 100, 100);      // Red
    pub const TOOL_PENDING: Color = Color::Rgb(120, 120, 140);    // Gray

    // Gradient stops for brand elements
    pub const GRADIENT_STOPS: [Color; 8] = [
        Color::Rgb(0, 200, 255),    // Cyan
        Color::Rgb(0, 180, 255),    // Light blue
        Color::Rgb(80, 160, 255),   // Blue
        Color::Rgb(140, 140, 255),  // Indigo
        Color::Rgb(180, 120, 255),  // Purple
        Color::Rgb(220, 100, 255),  // Magenta
        Color::Rgb(255, 120, 200),  // Pink
        Color::Rgb(255, 150, 150),  // Coral
    ];
}

/// Enhanced typography styles.
pub mod typography {
    use ratatui::style::{Color, Modifier, Style};

    /// Primary heading style — bold, bright.
    pub fn heading() -> Style {
        Style::default()
            .fg(Color::Rgb(240, 240, 245))
            .add_modifier(Modifier::BOLD)
    }

    /// Secondary heading style — medium weight.
    pub fn subheading() -> Style {
        Style::default()
            .fg(Color::Rgb(180, 180, 190))
            .add_modifier(Modifier::BOLD)
    }

    /// Body text style — normal weight.
    pub fn body() -> Style {
        Style::default()
            .fg(Color::Rgb(220, 220, 230))
    }

    /// Muted text style — for secondary information.
    pub fn muted() -> Style {
        Style::default()
            .fg(Color::Rgb(120, 120, 130))
    }

    /// Accent text style — for highlighted information.
    pub fn accent() -> Style {
        Style::default()
            .fg(Color::Rgb(0, 200, 255))
            .add_modifier(Modifier::BOLD)
    }

    /// Code text style — monospace feel.
    pub fn code() -> Style {
        Style::default()
            .fg(Color::Rgb(130, 200, 255))
            .add_modifier(Modifier::ITALIC)
    }

    /// Error text style.
    pub fn error() -> Style {
        Style::default()
            .fg(Color::Rgb(255, 100, 100))
            .add_modifier(Modifier::BOLD)
    }

    /// Success text style.
    pub fn success() -> Style {
        Style::default()
            .fg(Color::Rgb(80, 220, 120))
            .add_modifier(Modifier::BOLD)
    }

    /// Warning text style.
    pub fn warning() -> Style {
        Style::default()
            .fg(Color::Rgb(255, 180, 60))
            .add_modifier(Modifier::BOLD)
    }
}

/// Enhanced spacing tokens.
pub mod spacing {
    /// Extra small spacing — 1 cell.
    pub const XS: u16 = 1;
    /// Small spacing — 2 cells.
    pub const SM: u16 = 2;
    /// Medium spacing — 4 cells.
    pub const MD: u16 = 4;
    /// Large spacing — 8 cells.
    pub const LG: u16 = 8;
    /// Extra large spacing — 16 cells.
    pub const XL: u16 = 16;
}

/// Enhanced component styling.
pub mod components {
    use super::palette;
    use super::typography;
    use ratatui::{
        style::{Color, Modifier, Style},
        widgets::{Block, Borders, Padding},
    };

    /// Create a modern card block with subtle border.
    pub fn card_block(title: Option<&str>) -> Block<'static> {
        let mut block = Block::default()
            .borders(Borders::ALL)
            .border_type(ratatui::widgets::BorderType::Rounded)
            .border_style(Style::default().fg(palette::BORDER_PRIMARY))
            .padding(Padding::new(1, 1, 0, 0));

        if let Some(title) = title {
            block = block.title(Span::styled(
                format!(" {} ", title),
                Style::default()
                    .fg(palette::BRAND_ACCENT)
                    .add_modifier(Modifier::BOLD),
            ));
        }

        block
    }

    /// Create a focused card block with accent border.
    pub fn card_block_focused(title: Option<&str>) -> Block<'static> {
        let mut block = Block::default()
            .borders(Borders::ALL)
            .border_type(ratatui::widgets::BorderType::Rounded)
            .border_style(
                Style::default()
                    .fg(palette::BRAND_ACCENT)
                    .add_modifier(Modifier::BOLD),
            )
            .padding(Padding::new(1, 1, 0, 0));

        if let Some(title) = title {
            block = block.title(Span::styled(
                format!(" {} ", title),
                Style::default()
                    .fg(palette::BRAND_ACCENT_GLOW)
                    .add_modifier(Modifier::BOLD),
            ));
        }

        block
    }

    /// Create a modal block with elevated appearance.
    pub fn modal_block(title: Option<&str>) -> Block<'static> {
        let mut block = Block::default()
            .borders(Borders::ALL)
            .border_type(ratatui::widgets::BorderType::Rounded)
            .border_style(Style::default().fg(palette::BORDER_ACCENT))
            .padding(Padding::new(2, 2, 1, 1));

        if let Some(title) = title {
            block = block.title(Span::styled(
                format!(" {} ", title),
                Style::default()
                    .fg(palette::BRAND_ACCENT)
                    .add_modifier(Modifier::BOLD),
            ));
        }

        block
    }

    /// Create a status badge style.
    pub fn status_badge(text: &str, color: Color) -> Span<'static> {
        Span::styled(
            format!(" {} ", text),
            Style::default()
                .fg(color)
                .add_modifier(Modifier::BOLD),
        )
    }

    /// Create a key hint badge.
    pub fn key_hint(key: &str) -> Span<'static> {
        Span::styled(
            format!(" {} ", key),
            Style::default()
                .fg(palette::TEXT_PRIMARY)
                .bg(palette::BG_ELEVATED)
                .add_modifier(Modifier::BOLD),
        )
    }

    /// Create a separator line.
    pub fn separator(width: u16) -> Line<'static> {
        let chars: String = std::iter::repeat_n('─', width as usize).collect();
        Line::from(Span::styled(
            chars,
            Style::default().fg(palette::BORDER_SECONDARY),
        ))
    }

    /// Create a gradient separator line.
    pub fn gradient_separator(width: u16) -> Line<'static> {
        use ratatui::text::Span;

        let gradient = palette::GRADIENT_STOPS;
        let mut spans: Vec<Span<'static>> = Vec::new();
        let mut run_start = 0;

        for i in 0..width as usize {
            let t = i as f32 / width as f32;
            let seg = t * (gradient.len() - 1) as f32;
            let idx = seg.floor() as usize;
            let frac = seg - seg.floor();
            let c0 = gradient[idx.min(gradient.len() - 1)];
            let c1 = gradient[(idx + 1).min(gradient.len() - 1)];

            let color = lerp_color(c0, c1, frac);

            if i > 0 && color != spans.last().map(|s| s.style.fg.unwrap_or(Color::Reset)).unwrap_or(Color::Reset) {
                let n = i - run_start;
                let text: String = std::iter::repeat_n('━', n).collect();
                spans.push(Span::styled(
                    text,
                    Style::default().fg(spans.last().map(|s| s.style.fg.unwrap_or(Color::Reset)).unwrap_or(Color::Reset)),
                ));
                run_start = i;
            }
        }

        if run_start < width as usize {
            let n = width as usize - run_start;
            let text: String = std::iter::repeat_n('━', n).collect();
            spans.push(Span::styled(
                text,
                Style::default().fg(palette::BORDER_PRIMARY),
            ));
        }

        Line::from(spans)
    }
}

/// Linearly interpolate between two colors.
fn lerp_color(a: Color, b: Color, t: f32) -> Color {
    let (r1, g1, b1) = match a {
        Color::Rgb(r, g, b) => (r as f32, g as f32, b as f32),
        _ => return a,
    };
    let (r2, g2, b2) = match b {
        Color::Rgb(r, g, b) => (r as f32, g as f32, b as f32),
        _ => return b,
    };
    Color::Rgb(
        (r1 + (r2 - r1) * t) as u8,
        (g1 + (g2 - g1) * t) as u8,
        (b1 + (b2 - b1) * t) as u8,
    )
}

/// Animation easing functions.
pub mod easing {
    /// Linear easing.
    pub fn linear(t: f32) -> f32 {
        t
    }

    /// Ease-in quad.
    pub fn in_quad(t: f32) -> f32 {
        t * t
    }

    /// Ease-out quad.
    pub fn out_quad(t: f32) -> f32 {
        t * (2.0 - t)
    }

    /// Ease-in-out quad.
    pub fn in_out_quad(t: f32) -> f32 {
        if t < 0.5 {
            2.0 * t * t
        } else {
            -1.0 + (4.0 - 2.0 * t) * t
        }
    }

    /// Ease-out cubic.
    pub fn out_cubic(t: f32) -> f32 {
        let t1 = t - 1.0;
        t1 * t1 * t1 + 1.0
    }

    /// Ease-out elastic.
    pub fn out_elastic(t: f32) -> f32 {
        let c4 = (2.0 * std::f32::consts::PI) / 3.0;
        if t.abs() < f32::EPSILON {
            0.0
        } else if (t - 1.0).abs() < f32::EPSILON {
            1.0
        } else {
            (2.0_f32).powf(-10.0 * t) * ((t * 10.0 - 0.75) * c4).sin() + 1.0
        }
    }

    /// Ease-out bounce.
    pub fn out_bounce(t: f32) -> f32 {
        let n1 = 7.5625;
        let d1 = 2.75;
        if t < 1.0 / d1 {
            n1 * t * t
        } else if t < 2.0 / d1 {
            let t = t - 1.5 / d1;
            n1 * t * t + 0.75
        } else if t < 2.5 / d1 {
            let t = t - 2.25 / d1;
            n1 * t * t + 0.9375
        } else {
            let t = t - 2.625 / d1;
            n1 * t * t + 0.984375
        }
    }
}

/// Transition state for smooth animations.
#[derive(Debug, Clone)]
pub struct Transition {
    /// Current progress (0.0 to 1.0).
    pub progress: f32,
    /// Duration in seconds.
    pub duration: f32,
    /// Elapsed time in seconds.
    pub elapsed: f32,
    /// Easing function.
    pub easing: fn(f32) -> f32,
}

impl Transition {
    /// Create a new transition.
    pub fn new(duration: f32, easing: fn(f32) -> f32) -> Self {
        Self {
            progress: 0.0,
            duration,
            elapsed: 0.0,
            easing,
        }
    }

    /// Advance the transition by a delta time.
    pub fn advance(&mut self, dt: f32) {
        self.elapsed += dt;
        self.progress = (self.elapsed / self.duration).min(1.0);
    }

    /// Check if the transition is complete.
    pub fn is_complete(&self) -> bool {
        self.progress >= 1.0
    }

    /// Get the eased progress.
    pub fn eased_progress(&self) -> f32 {
        (self.easing)(self.progress)
    }

    /// Reset the transition.
    pub fn reset(&mut self) {
        self.progress = 0.0;
        self.elapsed = 0.0;
    }
}

/// Visual feedback effects.
pub mod effects {
    use super::palette;
    use ratatui::style::Color;

    /// Create a pulsing border color.
    pub fn pulsing_border(base: Color, elapsed: f32) -> Color {
        let pulse = (elapsed * 2.0).sin() * 0.5 + 0.5;
        let intensity = 0.7 + pulse * 0.3;
        match base {
            Color::Rgb(r, g, b) => Color::Rgb(
                (r as f32 * intensity) as u8,
                (g as f32 * intensity) as u8,
                (b as f32 * intensity) as u8,
            ),
            _ => base,
        }
    }

    /// Create a focus glow effect.
    pub fn focus_glow(base: Color, elapsed: f32) -> Color {
        let glow = (elapsed * 3.0).sin() * 0.5 + 0.5;
        let boost = 1.0 + glow * 0.2;
        match base {
            Color::Rgb(r, g, b) => Color::Rgb(
                (r as f32 * boost).min(255.0) as u8,
                (g as f32 * boost).min(255.0) as u8,
                (b as f32 * boost).min(255.0) as u8,
            ),
            _ => base,
        }
    }

    /// Create a shimmer effect for text.
    pub fn shimmer_text(text: &str, elapsed: f32) -> Vec<Span<'static>> {
        let gradient = palette::GRADIENT_STOPS;
        let mut spans: Vec<Span<'static>> = Vec::new();
        let mut run_color = gradient[0];
        let mut run_text = String::new();

        for (i, ch) in text.chars().enumerate() {
            let t = (i as f32 + elapsed * 10.0) / text.chars().count() as f32;
            let seg = t * (gradient.len() - 1) as f32;
            let idx = seg.floor() as usize;
            let frac = seg - seg.floor();
            let c0 = gradient[idx.min(gradient.len() - 1)];
            let c1 = gradient[(idx + 1).min(gradient.len() - 1)];
            let color = lerp_color(c0, c1, frac);

            if color != run_color && !run_text.is_empty() {
                spans.push(Span::styled(
                    std::mem::take(&mut run_text),
                    ratatui::style::Style::default()
                        .fg(run_color)
                        .add_modifier(ratatui::style::Modifier::BOLD),
                ));
                run_color = color;
            }
            run_text.push(ch);
        }

        if !run_text.is_empty() {
            spans.push(Span::styled(
                run_text,
                ratatui::style::Style::default()
                    .fg(run_color)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ));
        }

        spans
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_palette_colors_are_valid() {
        // Ensure all RGB colors have valid components
        assert!(matches!(palette::BRAND_ACCENT, Color::Rgb(_, _, _)));
        assert!(matches!(palette::AI_PRIMARY, Color::Rgb(_, _, _)));
        assert!(matches!(palette::USER_PRIMARY, Color::Rgb(_, _, _)));
    }

    #[test]
    fn test_typography_styles() {
        let heading = typography::heading();
        assert!(heading.add_modifier.contains(Modifier::BOLD));

        let muted = typography::muted();
        assert!(muted.fg.is_some());
    }

    #[test]
    fn test_easing_functions() {
        assert_eq!(easing::linear(0.5), 0.5);
        assert_eq!(easing::in_quad(0.0), 0.0);
        assert_eq!(easing::in_quad(1.0), 1.0);
        assert_eq!(easing::out_quad(0.0), 0.0);
        assert_eq!(easing::out_quad(1.0), 1.0);
    }

    #[test]
    fn test_transition() {
        let mut transition = Transition::new(1.0, easing::linear);
        assert!(!transition.is_complete());
        transition.advance(0.5);
        assert_eq!(transition.progress, 0.5);
        transition.advance(0.5);
        assert!(transition.is_complete());
    }

    #[test]
    fn test_lerp_color() {
        let a = Color::Rgb(0, 0, 0);
        let b = Color::Rgb(255, 255, 255);
        let mid = lerp_color(a, b, 0.5);
        assert_eq!(mid, Color::Rgb(127, 127, 127));
    }
}
