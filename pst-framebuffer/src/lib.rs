#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;

pub mod font;

use alloc::string::String;
use alloc::vec::Vec;
use alloc::collections::BTreeMap;

use pst_markout::parse::{self, Line, LineType};
use pst_markout::vnode::VNode;
use pst_markout::render;

use font::{GLYPH_WIDTH, GLYPH_HEIGHT};

#[derive(Debug, Clone, Copy)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self { Self { r, g, b } }
    pub const WHITE: Color = Color::rgb(255, 255, 255);
    pub const BLACK: Color = Color::rgb(0, 0, 0);
    pub const BLUE: Color = Color::rgb(59, 130, 246);
    pub const GRAY: Color = Color::rgb(100, 100, 100);
    pub const LIGHT_GRAY: Color = Color::rgb(220, 220, 220);
    pub const DARK_BG: Color = Color::rgb(30, 30, 30);
}

pub struct Framebuffer {
    pub pixels: Vec<u8>,
    pub width: usize,
    pub height: usize,
    pub stride: usize,
}

impl Framebuffer {
    pub fn new(width: usize, height: usize) -> Self {
        let stride = width * 4; // 32bpp BGRA
        Self {
            pixels: alloc::vec![0u8; stride * height],
            width,
            height,
            stride,
        }
    }

    pub fn clear(&mut self, color: Color) {
        for y in 0..self.height {
            for x in 0..self.width {
                self.set_pixel(x, y, color);
            }
        }
    }

    #[inline]
    pub fn set_pixel(&mut self, x: usize, y: usize, color: Color) {
        if x >= self.width || y >= self.height { return; }
        let offset = y * self.stride + x * 4;
        self.pixels[offset] = color.b;
        self.pixels[offset + 1] = color.g;
        self.pixels[offset + 2] = color.r;
        self.pixels[offset + 3] = 0xFF;
    }

    pub fn fill_rect(&mut self, x: usize, y: usize, w: usize, h: usize, color: Color) {
        for dy in 0..h {
            for dx in 0..w {
                self.set_pixel(x + dx, y + dy, color);
            }
        }
    }

    pub fn draw_char(&mut self, x: usize, y: usize, c: u8, fg: Color, bg: Color) {
        let glyph = font::glyph(c);
        for row in 0..GLYPH_HEIGHT {
            let bits = glyph[row];
            for col in 0..GLYPH_WIDTH {
                let color = if bits & (0x80 >> col) != 0 { fg } else { bg };
                self.set_pixel(x + col, y + row, color);
            }
        }
    }

    pub fn draw_text(&mut self, x: usize, y: usize, text: &str, fg: Color, bg: Color) {
        let mut cx = x;
        for c in text.bytes() {
            if cx + GLYPH_WIDTH > self.width { break; }
            self.draw_char(cx, y, c, fg, bg);
            cx += GLYPH_WIDTH;
        }
    }

    pub fn draw_text_transparent(&mut self, x: usize, y: usize, text: &str, fg: Color) {
        let mut cx = x;
        for c in text.bytes() {
            if cx + GLYPH_WIDTH > self.width { break; }
            let glyph = font::glyph(c);
            for row in 0..GLYPH_HEIGHT {
                let bits = glyph[row];
                for col in 0..GLYPH_WIDTH {
                    if bits & (0x80 >> col) != 0 {
                        self.set_pixel(cx + col, y + row, fg);
                    }
                }
            }
            cx += GLYPH_WIDTH;
        }
    }

    pub fn draw_hline(&mut self, x: usize, y: usize, w: usize, color: Color) {
        for dx in 0..w {
            self.set_pixel(x + dx, y, color);
        }
    }
}

/// Render a Markout document to a framebuffer.
/// This is the compositor. Same code path as HTML rendering,
/// but outputs pixels instead of strings.
pub fn render_markout(fb: &mut Framebuffer, markout: &str, bg: Color, fg: Color) {
    let lines = parse::parse(markout);
    let vdom = render::render(&lines);
    render_vnode(fb, &vdom, 16, 16, bg, fg);
}

fn render_vnode(fb: &mut Framebuffer, node: &VNode, x: usize, y: usize, bg: Color, fg: Color) -> usize {
    match node {
        VNode::Text(t) => {
            if !t.content.trim().is_empty() {
                fb.draw_text_transparent(x, y, &t.content, fg);
            }
            y + GLYPH_HEIGHT
        }
        VNode::Element(el) => {
            let mut cy = y;

            // Check for parametric positioning
            if let Some(style) = el.attrs.get("style") {
                if style.contains("position:absolute") {
                    let (px, py, pw, _ph) = parse_position(style);

                    // Render children at absolute position
                    for child in &el.children {
                        render_vnode(fb, child, x + px, y + py, bg, fg);
                    }
                    return cy;
                }
                if style.contains("position:relative") {
                    // Parametric container — children have absolute positions
                    for child in &el.children {
                        cy = render_vnode(fb, child, x, y, bg, fg);
                    }
                    return cy;
                }
            }

            let class = el.attrs.get("class").map(|s| s.as_str()).unwrap_or("");

            // Card — draw background + border
            if class.contains("mc-card") || class.contains("mc-nav") || class.contains("mc-header")
                || class.contains("mc-footer") || class.contains("mc-section") || class.contains("mc-form")
                || class.contains("mc-aside") {
                let cfg = parse_config(el.attrs.get("data-config").map(|s| s.as_str()).unwrap_or(""));
                let card_x = x;
                let card_y = cy;
                let pad = cfg.padding;
                let card_max_w = cfg.max_width.unwrap_or(fb.width - card_x * 2);
                let card_w = card_max_w.min(fb.width - card_x * 2);

                let mut inner_y = card_y + pad;
                for child in &el.children {
                    inner_y = render_vnode(fb, child, card_x + pad, inner_y, bg, fg);
                    inner_y += cfg.gap;
                }

                let card_h = if let Some(h) = cfg.height {
                    h
                } else {
                    inner_y - card_y + pad
                };

                fb.draw_hline(card_x, card_y, card_w, Color::GRAY);
                fb.draw_hline(card_x, card_y + card_h, card_w, Color::GRAY);
                for dy in 0..card_h {
                    fb.set_pixel(card_x, card_y + dy, Color::GRAY);
                    fb.set_pixel(card_x + card_w - 1, card_y + dy, Color::GRAY);
                }

                return card_y + card_h + 8;
            }

            // Row — horizontal layout
            if class.contains("mc-row") {
                let mut cx = x;
                let mut max_h = GLYPH_HEIGHT;
                for child in &el.children {
                    let (w, h) = render_vnode_inline(fb, child, cx, cy, bg, fg);
                    cx += w + 4;
                    if h > max_h { max_h = h; }
                }
                return cy + max_h + 2;
            }

            // Spacer — vertical gap
            if class.contains("mc-spacer") {
                return cy + 16;
            }

            // Input (text)
            if class.contains("mc-input") && !class.contains("mc-input-password") {
                let fw = 200;
                let fh = 22;
                let tab_w = 5;
                let tab_color = Color::rgb(59, 130, 246); // blue
                fb.fill_rect(x, cy, fw, fh, Color::rgb(50, 50, 55));
                fb.fill_rect(x, cy, tab_w, fh, tab_color);
                fb.fill_rect(x, cy, tab_w, 1, Color::rgb(99, 170, 255));
                fb.draw_hline(x, cy, fw, Color::rgb(70, 70, 75));
                fb.draw_hline(x, cy + fh - 1, fw, Color::rgb(40, 40, 45));
                return cy + fh + 4;
            }

            // Password
            if class.contains("mc-input-password") {
                let fw = 200;
                let fh = 22;
                let tab_w = 5;
                let tab_color = Color::rgb(239, 68, 68); // red
                fb.fill_rect(x, cy, fw, fh, Color::rgb(50, 50, 55));
                fb.fill_rect(x, cy, tab_w, fh, tab_color);
                fb.fill_rect(x, cy, tab_w, 1, Color::rgb(255, 108, 108));
                fb.draw_hline(x, cy, fw, Color::rgb(70, 70, 75));
                fb.draw_hline(x, cy + fh - 1, fw, Color::rgb(40, 40, 45));
                return cy + fh + 4;
            }

            // Checkbox
            if class.contains("mc-checkbox") {
                let tab_w = 5;
                let tab_color = Color::rgb(16, 185, 129); // green
                let fw = 22;
                let fh = 22;
                fb.fill_rect(x, cy, fw, fh, Color::rgb(50, 50, 55));
                fb.fill_rect(x, cy, tab_w, fh, tab_color);
                fb.fill_rect(x, cy, tab_w, 1, Color::rgb(56, 225, 169));
                fb.draw_hline(x, cy, fw, Color::rgb(70, 70, 75));
                fb.draw_hline(x, cy + fh - 1, fw, Color::rgb(40, 40, 45));
                // Empty checkbox square
                fb.fill_rect(x + tab_w + 3, cy + 3, 14, 14, Color::rgb(40, 40, 45));
                fb.draw_hline(x + tab_w + 3, cy + 3, 14, Color::rgb(70, 70, 75));
                return cy + fh + 4;
            }

            // Button
            if class.contains("mc-button") {
                let label = text_content(node);
                let w = label.len() * GLYPH_WIDTH + 24;
                let h = GLYPH_HEIGHT + 12;
                let btn_color = style_color(class).unwrap_or(Color::rgb(59, 130, 246));
                let text_fg = style_fg(class);
                fb.fill_rect(x, cy, w, h, btn_color);
                fb.draw_hline(x, cy, w, Color::rgb(
                    btn_color.r.saturating_add(40),
                    btn_color.g.saturating_add(40),
                    btn_color.b.saturating_add(40),
                ));
                fb.draw_hline(x, cy + h - 1, w, Color::rgb(
                    btn_color.r.saturating_sub(30),
                    btn_color.g.saturating_sub(30),
                    btn_color.b.saturating_sub(30),
                ));
                fb.draw_text(x + 12, cy + 6, &label, text_fg, btn_color);
                return cy + h + 6;
            }

            // Divider
            if class.contains("mc-divider") {
                fb.draw_hline(x, cy + 4, fb.width - x * 2, Color::GRAY);
                return cy + 12;
            }

            // Label / default
            if class.contains("mc-label") {
                let content = text_content(node);
                fb.draw_text_transparent(x, cy, &content, fg);
                return cy + GLYPH_HEIGHT;
            }

            // Generic container
            for child in &el.children {
                cy = render_vnode(fb, child, x, cy, bg, fg);
                cy += 2;
            }
            cy
        }
    }
}

fn col_width(el: &VElement, container_w: usize) -> Option<usize> {
    // Check responsive breakpoints first
    if let Some(resp_str) = el.attrs.get("data-responsive") {
        let bp = if container_w < 640 { "sm" }
            else if container_w < 1024 { "md" }
            else if container_w < 1280 { "lg" }
            else { "xl" };

        // Find the best matching breakpoint (largest that fits)
        let breakpoints = ["sm", "md", "lg", "xl"];
        let bp_idx = breakpoints.iter().position(|&b| b == bp).unwrap_or(0);

        for check_idx in (0..=bp_idx).rev() {
            let check_bp = breakpoints[check_idx];
            for entry in resp_str.split(';') {
                if let Some(rest) = entry.strip_prefix(check_bp) {
                    if let Some(col_part) = rest.strip_prefix(':') {
                        let parts: Vec<&str> = col_part.split(',').collect();
                        if parts.len() == 2 {
                            if let (Ok(span), Ok(total)) = (parts[0].parse::<usize>(), parts[1].parse::<usize>()) {
                                if total > 0 { return Some((container_w * span) / total); }
                            }
                        }
                    }
                }
            }
        }
    }

    // Fall back to static col
    if let Some(col_str) = el.attrs.get("data-col") {
        let parts: Vec<&str> = col_str.split(',').collect();
        if parts.len() == 2 {
            if let (Ok(span), Ok(total)) = (parts[0].parse::<usize>(), parts[1].parse::<usize>()) {
                if total > 0 { return Some((container_w * span) / total); }
            }
        }
    }
    None
}

fn render_vnode_inline(fb: &mut Framebuffer, node: &VNode, x: usize, y: usize, bg: Color, fg: Color) -> (usize, usize) {
    match node {
        VNode::Text(t) => {
            let text = t.content.trim();
            if !text.is_empty() {
                fb.draw_text_transparent(x, y + 4, text, fg);
            }
            (text.len() * GLYPH_WIDTH, GLYPH_HEIGHT)
        }
        VNode::Element(el) => {
            let class = el.attrs.get("class").map(|s| s.as_str()).unwrap_or("");

            if class.contains("mc-input") && !class.contains("mc-input-password") {
                let fw = col_width(el, fb.width.saturating_sub(x * 2)).unwrap_or(200);
                let fh = 22;
                let tab_w = 5;
                fb.fill_rect(x, y, fw, fh, Color::rgb(50, 50, 55));
                fb.fill_rect(x, y, tab_w, fh, Color::rgb(59, 130, 246));
                fb.fill_rect(x, y, tab_w, 1, Color::rgb(99, 170, 255));
                fb.draw_hline(x, y, fw, Color::rgb(70, 70, 75));
                fb.draw_hline(x, y + fh - 1, fw, Color::rgb(40, 40, 45));
                return (fw, fh);
            }

            if class.contains("mc-input-password") {
                let fw = 200;
                let fh = 22;
                let tab_w = 5;
                fb.fill_rect(x, y, fw, fh, Color::rgb(50, 50, 55));
                fb.fill_rect(x, y, tab_w, fh, Color::rgb(239, 68, 68));
                fb.fill_rect(x, y, tab_w, 1, Color::rgb(255, 108, 108));
                fb.draw_hline(x, y, fw, Color::rgb(70, 70, 75));
                fb.draw_hline(x, y + fh - 1, fw, Color::rgb(40, 40, 45));
                return (fw, fh);
            }

            if class.contains("mc-checkbox") {
                let fh = 22;
                let tab_w = 5;
                fb.fill_rect(x, y, 22, fh, Color::rgb(50, 50, 55));
                fb.fill_rect(x, y, tab_w, fh, Color::rgb(16, 185, 129));
                fb.fill_rect(x, y, tab_w, 1, Color::rgb(56, 225, 169));
                fb.draw_hline(x, y, 22, Color::rgb(70, 70, 75));
                fb.draw_hline(x, y + fh - 1, 22, Color::rgb(40, 40, 45));
                fb.fill_rect(x + tab_w + 3, y + 3, 14, 14, Color::rgb(40, 40, 45));
                fb.draw_hline(x + tab_w + 3, y + 3, 14, Color::rgb(70, 70, 75));
                return (22, fh);
            }

            if class.contains("mc-button") {
                let label = text_content(node);
                let w = label.len() * GLYPH_WIDTH + 24;
                let h = GLYPH_HEIGHT + 12;
                let btn_color = style_color(class).unwrap_or(Color::rgb(59, 130, 246));
                fb.fill_rect(x, y, w, h, btn_color);
                fb.draw_hline(x, y, w, Color::rgb(
                    btn_color.r.saturating_add(40), btn_color.g.saturating_add(40), btn_color.b.saturating_add(40)));
                fb.draw_hline(x, y + h - 1, w, Color::rgb(
                    btn_color.r.saturating_sub(30), btn_color.g.saturating_sub(30), btn_color.b.saturating_sub(30)));
                fb.draw_text(x + 12, y + 6, &label, Color::WHITE, btn_color);
                return (w, h);
            }

            if class.contains("mc-label") {
                let content = text_content(node);
                fb.draw_text_transparent(x, y + 4, &content, fg);
                return (content.len() * GLYPH_WIDTH, GLYPH_HEIGHT);
            }

            if class.contains("mc-radio") {
                let fh = 22;
                let tab_w = 5;
                fb.fill_rect(x, y, 22, fh, Color::rgb(50, 50, 55));
                fb.fill_rect(x, y, tab_w, fh, Color::rgb(168, 85, 247));
                fb.fill_rect(x, y, tab_w, 1, Color::rgb(208, 125, 255));
                fb.draw_hline(x, y, 22, Color::rgb(70, 70, 75));
                fb.draw_hline(x, y + fh - 1, 22, Color::rgb(40, 40, 45));
                // Circle outline
                for dx in 2..12 { fb.set_pixel(x + tab_w + 3 + dx, y + 4, Color::rgb(100, 100, 110)); }
                for dx in 2..12 { fb.set_pixel(x + tab_w + 3 + dx, y + 17, Color::rgb(100, 100, 110)); }
                return (22, fh);
            }

            if class.contains("mc-select") {
                let fw = 160;
                let fh = 22;
                let tab_w = 5;
                fb.fill_rect(x, y, fw, fh, Color::rgb(50, 50, 55));
                fb.fill_rect(x, y, tab_w, fh, Color::rgb(245, 158, 11));
                fb.fill_rect(x, y, tab_w, 1, Color::rgb(255, 198, 51));
                fb.draw_hline(x, y, fw, Color::rgb(70, 70, 75));
                fb.draw_hline(x, y + fh - 1, fw, Color::rgb(40, 40, 45));
                // Down arrow
                fb.draw_text(x + fw - 16, y + 5, "v", Color::rgb(150, 150, 150), Color::rgb(50, 50, 55));
                return (fw, fh);
            }

            if class.contains("mc-textarea") {
                let fw = 250;
                let fh = 60;
                let tab_w = 5;
                fb.fill_rect(x, y, fw, fh, Color::rgb(50, 50, 55));
                fb.fill_rect(x, y, tab_w, fh, Color::rgb(59, 130, 246));
                fb.fill_rect(x, y, tab_w, 1, Color::rgb(99, 170, 255));
                fb.draw_hline(x, y, fw, Color::rgb(70, 70, 75));
                fb.draw_hline(x, y + fh - 1, fw, Color::rgb(40, 40, 45));
                return (fw, fh);
            }

            if class.contains("mc-image") {
                let fw = 64;
                let fh = 48;
                fb.fill_rect(x, y, fw, fh, Color::rgb(60, 60, 65));
                fb.draw_hline(x, y, fw, Color::rgb(80, 80, 85));
                fb.draw_hline(x, y + fh - 1, fw, Color::rgb(40, 40, 45));
                fb.draw_text(x + 16, y + 18, "IMG", Color::rgb(100, 100, 110), Color::rgb(60, 60, 65));
                return (fw, fh);
            }

            if class.contains("mc-link") {
                let label = text_content(node);
                fb.draw_text_transparent(x, y + 4, &label, Color::rgb(96, 165, 250));
                // Underline
                fb.draw_hline(x, y + 4 + GLYPH_HEIGHT, label.len() * GLYPH_WIDTH, Color::rgb(96, 165, 250));
                return (label.len() * GLYPH_WIDTH, GLYPH_HEIGHT + 2);
            }

            if class.contains("mc-pill") {
                let label = text_content(node);
                let pw = label.len() * GLYPH_WIDTH + 12;
                let ph = GLYPH_HEIGHT + 6;
                fb.fill_rect(x, y, pw, ph, Color::rgb(55, 65, 81));
                fb.draw_hline(x, y, pw, Color::rgb(75, 85, 101));
                fb.draw_text(x + 6, y + 3, &label, Color::rgb(200, 200, 210), Color::rgb(55, 65, 81));
                return (pw, ph);
            }

            if class.contains("mc-badge") {
                let label = text_content(node);
                let bw = label.len() * GLYPH_WIDTH + 8;
                let bh = GLYPH_HEIGHT + 4;
                fb.fill_rect(x, y, bw, bh, Color::rgb(239, 68, 68));
                fb.draw_text(x + 4, y + 2, &label, Color::WHITE, Color::rgb(239, 68, 68));
                return (bw, bh);
            }

            if class.contains("mc-progress") {
                let label = text_content(node);
                let pct: usize = label.trim().parse().unwrap_or(50);
                let pw = 200;
                let ph = 12;
                fb.fill_rect(x, y + 4, pw, ph, Color::rgb(40, 40, 45));
                let fill_w = (pw * pct) / 100;
                fb.fill_rect(x, y + 4, fill_w, ph, Color::rgb(59, 130, 246));
                fb.draw_hline(x, y + 4, pw, Color::rgb(60, 60, 65));
                return (pw, ph + 8);
            }

            if class.contains("mc-sparkline") {
                let sw = 80;
                let sh = 20;
                fb.fill_rect(x, y, sw, sh, Color::rgb(40, 40, 45));
                // Placeholder zigzag
                for i in 0..sw {
                    let py = y + sh / 2 + ((i * 7) % sh).min(sh - 2) - sh / 4;
                    fb.set_pixel(x + i, py, Color::rgb(16, 185, 129));
                }
                return (sw, sh);
            }

            if class.contains("mc-spacer") {
                return (8, 0);
            }

            if class.contains("mc-divider") {
                let w = fb.width.saturating_sub(x * 2);
                fb.draw_hline(x, y + 4, w, Color::GRAY);
                return (w, 12);
            }

            // Generic: render children inline
            let mut cx = x;
            let mut max_h = 0;
            for child in &el.children {
                let (cw, ch) = render_vnode_inline(fb, child, cx, y, bg, fg);
                cx += cw + 4;
                if ch > max_h { max_h = ch; }
            }
            (cx - x, max_h)
        }
    }
}

struct ContainerConfig {
    padding: usize,
    width: Option<usize>,
    max_width: Option<usize>,
    height: Option<usize>,
    gap: usize,
}

fn parse_config(config: &str) -> ContainerConfig {
    let mut cfg = ContainerConfig { padding: 12, width: None, max_width: None, height: None, gap: 4 };
    for part in config.split(|c: char| c == ',' || c == ' ') {
        let part = part.trim();
        if let Some((key, val)) = part.split_once(':') {
            let px = parse_px_value(val);
            match key {
                "padding" => cfg.padding = px,
                "width" => cfg.width = Some(px),
                "max-width" => cfg.max_width = Some(px),
                "height" => cfg.height = Some(px),
                "max-height" => cfg.height = Some(px),
                "gap" => cfg.gap = px,
                _ => {}
            }
        }
    }
    cfg
}

fn parse_px_value(s: &str) -> usize {
    let s = s.trim().trim_end_matches("px").trim_end_matches("rem");
    if let Some(dot) = s.find('.') {
        s[..dot].parse().unwrap_or(0)
    } else {
        s.parse().unwrap_or(0)
    }
}

fn style_color(class: &str) -> Option<Color> {
    if class.contains("mc-primary") { Some(Color::rgb(59, 130, 246)) }
    else if class.contains("mc-secondary") { Some(Color::rgb(107, 114, 128)) }
    else if class.contains("mc-danger") { Some(Color::rgb(239, 68, 68)) }
    else if class.contains("mc-warning") { Some(Color::rgb(245, 158, 11)) }
    else if class.contains("mc-info") { Some(Color::rgb(6, 182, 212)) }
    else if class.contains("mc-dark") { Some(Color::rgb(30, 30, 30)) }
    else if class.contains("mc-light") { Some(Color::rgb(229, 231, 235)) }
    else if class.contains("mc-outline") { Some(Color::rgb(107, 114, 128)) }
    else if class.contains("mc-ghost") { Some(Color::rgb(55, 65, 81)) }
    else { None }
}

fn style_fg(class: &str) -> Color {
    if class.contains("mc-dark") { Color::WHITE }
    else if class.contains("mc-light") { Color::BLACK }
    else if class.contains("mc-ghost") { Color::rgb(180, 180, 180) }
    else { Color::WHITE }
}

fn size_scale(class: &str) -> usize {
    for i in 1..=9u8 {
        let needle = alloc::format!("mc-size-{}", i);
        if class.contains(&needle) { return i as usize; }
    }
    5 // default
}

fn text_content(node: &VNode) -> String {
    match node {
        VNode::Text(t) => t.content.clone(),
        VNode::Element(el) => {
            let mut s = String::new();
            for child in &el.children {
                s.push_str(&text_content(child));
            }
            s
        }
    }
}

fn parse_position(style: &str) -> (usize, usize, usize, usize) {
    let mut x = 0usize;
    let mut y = 0usize;
    let mut w = 0usize;
    let mut h = 0usize;

    for part in style.split(';') {
        let part = part.trim();
        if let Some(val) = part.strip_prefix("left:") {
            x = parse_px(val);
        } else if let Some(val) = part.strip_prefix("top:") {
            y = parse_px(val);
        } else if let Some(val) = part.strip_prefix("width:") {
            w = parse_px(val);
        } else if let Some(val) = part.strip_prefix("height:") {
            h = parse_px(val);
        }
    }
    (x, y, w, h)
}

fn parse_px(s: &str) -> usize {
    let s = s.trim().trim_end_matches("px");
    // Handle float values like "120.0"
    if let Some(dot) = s.find('.') {
        s[..dot].parse().unwrap_or(0)
    } else {
        s.parse().unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_framebuffer_create() {
        let fb = Framebuffer::new(640, 480);
        assert_eq!(fb.pixels.len(), 640 * 480 * 4);
    }

    #[test]
    fn test_draw_char() {
        let mut fb = Framebuffer::new(64, 32);
        fb.clear(Color::BLACK);
        fb.draw_char(0, 0, b'A', Color::WHITE, Color::BLACK);
        let mut has_white = false;
        for row in 0..GLYPH_HEIGHT {
            for col in 0..GLYPH_WIDTH {
                let off = row * fb.stride + col * 4;
                if fb.pixels[off + 2] == 255 { has_white = true; break; }
            }
            if has_white { break; }
        }
        assert!(has_white);
    }

    #[test]
    fn test_draw_text() {
        let mut fb = Framebuffer::new(640, 480);
        fb.clear(Color::DARK_BG);
        fb.draw_text(10, 10, "PST OS", Color::WHITE, Color::DARK_BG);
        // Check the text area for white pixels
        let mut has_text = false;
        for row in 10..10 + GLYPH_HEIGHT {
            for col in 10..10 + 6 * GLYPH_WIDTH {
                let off = row * fb.stride + col * 4;
                if fb.pixels[off + 2] == 255 { has_text = true; break; }
            }
            if has_text { break; }
        }
        assert!(has_text);
    }

    #[test]
    fn test_render_markout_to_framebuffer() {
        let mut fb = Framebuffer::new(800, 600);
        fb.clear(Color::DARK_BG);

        render_markout(&mut fb, "\
@card
| Parallel String Theory OS
@parametric
| {label:title \"PST OS v0.1\"}
| {label:status \"Running\" center-x:title gap-y:16}
@end parametric
| The thesis is proven.
@end card",
            Color::DARK_BG, Color::WHITE);

        // Check that pixels were written (not all dark background)
        let non_bg = fb.pixels.chunks(4)
            .filter(|p| p[0] != 30 || p[1] != 30 || p[2] != 30)
            .count();
        assert!(non_bg > 100, "expected rendered pixels, got {}", non_bg);
    }

    #[test]
    fn test_parametric_positions_applied() {
        let mut fb = Framebuffer::new(800, 600);
        fb.clear(Color::BLACK);

        render_markout(&mut fb, "\
@parametric
| {label:a \"Top\"}
| {label:b \"Bottom\" left:a gap-y:32}
@end parametric",
            Color::BLACK, Color::WHITE);

        // "Top" renders at base y (16 padding + 0 from solver = 16)
        // "Bottom" renders at y=16 + 24 (glyph height) + 32 (gap) = 72
        // Scan a wide band for white pixels below the "Top" label area
        let mut has_bottom = false;
        for row in 50..100 {
            for col in 0..fb.width {
                let off = row * fb.stride + col * 4;
                if fb.pixels[off + 2] == 255 { has_bottom = true; break; }
            }
            if has_bottom { break; }
        }
        assert!(has_bottom, "expected 'Bottom' text rendered below 'Top' via gap-y:32");
    }
}
