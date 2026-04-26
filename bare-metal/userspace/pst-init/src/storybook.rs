use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use alloc::format;

use crate::ps2::{Ps2, InputEvent};
use crate::serial_print;
use pst_framebuffer::{Framebuffer, Color, render_markout};

struct Knob {
    name: &'static str,
    options: &'static [&'static str],
    selected: usize,
}

impl Knob {
    fn val(&self) -> &'static str { self.options[self.selected] }
    fn cycle(&mut self) { self.selected = (self.selected + 1) % self.options.len(); }
}

struct Story {
    title: &'static str,
    template: &'static str,
    knobs: Vec<Knob>,
}

impl Story {
    fn render_source(&self) -> String {
        let mut s = String::from(self.template);
        for knob in &self.knobs {
            let placeholder = format!("{{{}}}", knob.name);
            while s.contains(&placeholder) {
                s = s.replacen(&placeholder, knob.val(), 1);
            }
        }
        s
    }
}

fn make_stories() -> Vec<Story> {
    let mut stories = Vec::new();

    stories.push(Story {
        title: "Label",
        template: "| {label:a \"{text}\" {style} {size}}",
        knobs: vec![
            Knob { name: "text", options: &["Hello World", "PST OS", "Parallel Strings", "Warning!"], selected: 0 },
            Knob { name: "style", options: &["", "primary", "danger", "success", "warning", "ghost", "muted", "accent"], selected: 0 },
            Knob { name: "size", options: &["", "sm", "lg", "xl"], selected: 0 },
        ],
    });

    stories.push(Story {
        title: "Button",
        template: "| {button:a \"{text}\" {style} {size}}",
        knobs: vec![
            Knob { name: "text", options: &["Click Me", "Submit", "Delete", "Save"], selected: 0 },
            Knob { name: "style", options: &["", "primary", "danger", "success", "warning", "ghost", "outline"], selected: 0 },
            Knob { name: "size", options: &["", "sm", "lg", "xl"], selected: 0 },
        ],
    });

    stories.push(Story {
        title: "Input",
        template: "| {{input}:field}  {label}",
        knobs: vec![
            Knob { name: "input", options: &["input", "password", "textarea"], selected: 0 },
            Knob { name: "label", options: &["Username", "Email", "Search", "Notes"], selected: 0 },
        ],
    });

    stories.push(Story {
        title: "Checkbox & Radio",
        template: "| {{kind}:opt}  {label}",
        knobs: vec![
            Knob { name: "kind", options: &["checkbox", "radio"], selected: 0 },
            Knob { name: "label", options: &["Accept terms", "Subscribe", "Enable dark mode", "Remember me"], selected: 0 },
        ],
    });

    stories.push(Story {
        title: "Badge & Pill",
        template: "| {{kind}:a \"{text}\" {style}}",
        knobs: vec![
            Knob { name: "kind", options: &["badge", "pill"], selected: 0 },
            Knob { name: "text", options: &["v1.0", "stable", "beta", "error", "new"], selected: 0 },
            Knob { name: "style", options: &["", "primary", "success", "danger", "warning"], selected: 0 },
        ],
    });

    stories.push(Story {
        title: "Progress",
        template: "| {progress:bar {style}}  {label}",
        knobs: vec![
            Knob { name: "style", options: &["", "primary", "success", "danger", "warning"], selected: 0 },
            Knob { name: "label", options: &["Loading...", "Upload", "CPU Usage", "Disk I/O"], selected: 0 },
        ],
    });

    stories.push(Story {
        title: "Select",
        template: "| {select:s \"{options}\"}  {label}",
        knobs: vec![
            Knob { name: "options", options: &["Red,Green,Blue", "S,M,L,XL", "Low,Medium,High", "Mon,Tue,Wed,Thu,Fri"], selected: 0 },
            Knob { name: "label", options: &["Color", "Size", "Priority", "Day"], selected: 0 },
        ],
    });

    stories.push(Story {
        title: "Grid Layout",
        template: "\
| {input:a {cols_a}}  {input:b {cols_b}}
| {button:c \"Action\" primary {cols_c}}",
        knobs: vec![
            Knob { name: "cols_a", options: &["col-6", "col-4", "col-8", "col-3"], selected: 0 },
            Knob { name: "cols_b", options: &["col-6", "col-8", "col-4", "col-9"], selected: 0 },
            Knob { name: "cols_c", options: &["col-12", "col-6", "col-4", "col-8"], selected: 0 },
        ],
    });

    stories.push(Story {
        title: "Styles Gallery",
        template: "\
| {button:a \"{style}\" {style}}  {label:b \"{style}\" {style}}  {badge:c \"{style}\" {style}}",
        knobs: vec![
            Knob { name: "style", options: &["primary", "danger", "success", "warning", "ghost", "muted", "outline", "accent"], selected: 0 },
        ],
    });

    stories.push(Story {
        title: "Card",
        template: "\
@card
| {label:t \"{title}\" primary}
| {divider:d}
| {label:m \"{body}\"}
| {progress:p {style}}
| {button:act \"{action}\" {style}}
@end card",
        knobs: vec![
            Knob { name: "title", options: &["Dashboard", "Settings", "Profile", "Analytics"], selected: 0 },
            Knob { name: "body", options: &["System nominal", "3 warnings", "All clear", "Check logs"], selected: 0 },
            Knob { name: "action", options: &["View Details", "Save", "Dismiss", "Retry"], selected: 0 },
            Knob { name: "style", options: &["primary", "success", "danger", "warning"], selected: 0 },
        ],
    });

    stories.push(Story {
        title: "Form",
        template: "\
@card
| {label:t \"{title}\" primary}
| {divider:d}
| {input:user}  Username
| {{field}:secret}  {field_label}
| {checkbox:agree}  {check_label}
| {spacer:s}
| {button:go \"{action}\" {style}}
@end card",
        knobs: vec![
            Knob { name: "title", options: &["Sign In", "Register", "Reset Password", "Preferences"], selected: 0 },
            Knob { name: "field", options: &["password", "input"], selected: 0 },
            Knob { name: "field_label", options: &["Password", "Email", "Token", "Code"], selected: 0 },
            Knob { name: "check_label", options: &["Remember me", "Accept terms", "Stay signed in", "Enable 2FA"], selected: 0 },
            Knob { name: "action", options: &["Sign In", "Create Account", "Reset", "Save"], selected: 0 },
            Knob { name: "style", options: &["primary", "success", "danger", "warning"], selected: 0 },
        ],
    });

    stories.push(Story {
        title: "Divider & Spacer",
        template: "\
| {label:a \"Above\" {style}}
| {{separator}:sep}
| {label:b \"Below\" {style}}",
        knobs: vec![
            Knob { name: "separator", options: &["divider", "spacer"], selected: 0 },
            Knob { name: "style", options: &["", "primary", "muted", "danger"], selected: 0 },
        ],
    });

    stories.push(Story {
        title: "Sparkline",
        template: "| {sparkline:s {style}}  {label}",
        knobs: vec![
            Knob { name: "style", options: &["", "primary", "success", "danger", "warning"], selected: 0 },
            Knob { name: "label", options: &["CPU", "Memory", "Network", "Latency"], selected: 0 },
        ],
    });

    stories.push(Story {
        title: "Link",
        template: "| {link:l \"{text}\" href:\"/pst/{href}\" {style}}",
        knobs: vec![
            Knob { name: "text", options: &["Documentation", "Source Code", "Downloads", "API Reference"], selected: 0 },
            Knob { name: "href", options: &["docs", "src", "releases", "api"], selected: 0 },
            Knob { name: "style", options: &["", "primary", "danger", "muted"], selected: 0 },
        ],
    });

    stories.push(Story {
        title: "All Components",
        template: "\
| {label:l \"Label\" {style}}  {button:b \"Button\" {style}}  {badge:bg \"tag\" {style}}
| {input:i}  {password:p}  {select:s \"A,B,C\"}
| {checkbox:c}  Check  {radio:r}  Radio  {pill:pl \"pill\" {style}}
| {progress:pr {style}}  {sparkline:sp}
| {divider:d}
| {link:lk \"Link\" href:\"/\"}",
        knobs: vec![
            Knob { name: "style", options: &["primary", "danger", "success", "warning", "ghost", "muted"], selected: 0 },
        ],
    });

    // === Combo plates ======================================================

    stories.push(Story {
        title: "Combo: Login Page",
        template: "\
@card
| {label:t \"Welcome Back\" primary lg}
| {label:sub \"Sign in to your account\" muted}
| {divider:d}
| {input:email}  Email address
| {password:pass}  Password
| {checkbox:remember}  Remember me
| {spacer:s}
| {button:signin \"Sign In\" primary col-12}
| {spacer:s2}
| {link:forgot \"Forgot password?\" muted}  {link:register \"Create account\" primary}
@end card",
        knobs: vec![],
    });

    stories.push(Story {
        title: "Combo: Dashboard",
        template: "\
| {label:t \"System Overview\" primary lg}
| {divider:d}
| {label:cpu \"CPU\" muted}  {progress:cpu primary}  {sparkline:cpu_h}
| {label:mem \"Memory\" muted}  {progress:mem success}  {sparkline:mem_h}
| {label:disk \"Disk\" muted}  {progress:disk warning}  {sparkline:disk_h}
| {label:net \"Network\" muted}  {progress:net primary}  {sparkline:net_h}
| {divider:d2}
| {badge:up \"online\" success}  {badge:load \"load: 0.42\" primary}  {badge:procs \"47 processes\"}
| {pill:rust \"Rust\" primary}  {pill:sel4 \"seL4\" success}  {pill:arch \"x86_64\" warning}",
        knobs: vec![],
    });

    stories.push(Story {
        title: "Combo: Settings",
        template: "\
@card
| {label:t \"Preferences\" primary}
| {divider:d}
| {checkbox:dark}  Enable dark mode
| {checkbox:notify}  Desktop notifications
| {checkbox:sound}  Sound effects
| {checkbox:auto}  Auto-save on exit
| {divider:d2}
| {label:lang \"Language\" muted}
| {select:lang \"English,Spanish,French,German,Japanese\"}
| {label:theme \"Accent Color\" muted}
| {select:theme \"Blue,Green,Purple,Orange,Red\"}
| {spacer:s}
| {button:cancel \"Cancel\" ghost col-6}  {button:save \"Save Changes\" primary col-6}
@end card",
        knobs: vec![],
    });

    stories.push(Story {
        title: "Combo: Profile",
        template: "\
@card
| {label:name \"Alice Chen\" primary lg}
| {pill:role \"Admin\" danger}  {pill:team \"Platform\" primary}  {badge:id \"#1042\"}
| {divider:d}
| {label:em \"Email\" muted}
| {input:email}
| {label:bio \"Bio\" muted}
| {textarea:bio}
| {divider:d2}
| {label:sec \"Two-Factor Authentication\" muted}
| {radio:tfa}  SMS
| {radio:tfa2}  Authenticator App
| {radio:tfa3}  Hardware Key
| {spacer:s}
| {button:update \"Update Profile\" primary}
@end card",
        knobs: vec![],
    });

    stories.push(Story {
        title: "Combo: File Manager",
        template: "\
| {label:path \"/pst/documents\" muted}  {button:up \"..\" ghost}  {button:new \"+ New\" primary}
| {divider:d}
| {checkbox:f1}  {label:n1 \"desktop.md\"}  {badge:s1 \"1.2K\" muted}  {pill:t1 \"md\" primary}
| {checkbox:f2}  {label:n2 \"welcome.md\"}  {badge:s2 \"842B\" muted}  {pill:t2 \"md\" primary}
| {checkbox:f3}  {label:n3 \"theme.md\"}  {badge:s3 \"256B\" muted}  {pill:t3 \"md\" primary}
| {checkbox:f4}  {label:n4 \"notes.txt\"}  {badge:s4 \"4.1K\" muted}  {pill:t4 \"txt\" warning}
| {checkbox:f5}  {label:n5 \"config.toml\"}  {badge:s5 \"512B\" muted}  {pill:t5 \"toml\" success}
| {divider:d2}
| {label:sel \"0 selected\" muted}  {button:del \"Delete\" danger}  {button:dl \"Download\" ghost}",
        knobs: vec![],
    });

    stories.push(Story {
        title: "Combo: Processes",
        template: "\
| {label:t \"Process Table\" primary lg}
| {divider:d}
| {badge:pid0 \"PID 0\" muted}  {label:p0 \"init\"}  {pill:s0 \"running\" success}  {progress:c0}
| {badge:pid1 \"PID 1\" muted}  {label:p1 \"cryptod\"}  {pill:s1 \"running\" success}  {progress:c1}
| {badge:pid2 \"PID 2\" muted}  {label:p2 \"vfs\"}  {pill:s2 \"running\" success}  {progress:c2}
| {badge:pid3 \"PID 3\" muted}  {label:p3 \"netd\"}  {pill:s3 \"blocked\" warning}  {progress:c3}
| {badge:pid4 \"PID 4\" muted}  {label:p4 \"compositor\"}  {pill:s4 \"ready\" primary}  {progress:c4}
| {badge:pid5 \"PID 5\" muted}  {label:p5 \"driverd\"}  {pill:s5 \"tombstoned\" danger}
| {divider:d2}
| {label:total \"6 processes\" muted}  {badge:live \"5 live\" success}  {badge:dead \"1 tombstoned\" danger}",
        knobs: vec![],
    });

    stories.push(Story {
        title: "Combo: Error Dialog",
        template: "\
@card
| {label:icon \"!!\" danger lg}  {label:t \"Constraint Violation\" danger lg}
| {divider:d}
| {label:msg \"Process 'netd' exceeded its scheduling budget\"}
| {label:detail \"The watchdog has recorded 3 violations in the last 60 ticks.\" muted}
| {spacer:s}
| {label:action \"Recommended action:\" muted}
| {radio:r1}  Restart process
| {radio:r2}  Increase budget
| {radio:r3}  Tombstone and remove
| {spacer:s2}
| {button:dismiss \"Dismiss\" ghost col-4}  {button:apply \"Apply\" danger col-4}  {button:ignore \"Ignore All\" muted col-4}
@end card",
        knobs: vec![],
    });

    stories.push(Story {
        title: "Combo: Network",
        template: "\
| {label:t \"Network\" primary lg}
| {badge:proto \"TCP/IP\" primary}  {badge:stack \"smoltcp\" success}  {badge:iface \"virtio-net\"}
| {divider:d}
| {label:rx \"RX\" muted}  {sparkline:rx}  {badge:rxb \"1.2 MB\" primary}
| {label:tx \"TX\" muted}  {sparkline:tx}  {badge:txb \"340 KB\" primary}
| {label:lat \"Latency\" muted}  {sparkline:lat}  {badge:ms \"12ms\" success}
| {divider:d2}
| {label:dns \"DNS\" muted}  {input:dns}
| {label:gw \"Gateway\" muted}  {input:gw}
| {spacer:s}
| {button:ping \"Ping\" primary}  {button:trace \"Traceroute\" ghost}  {button:reset \"Reset\" danger}",
        knobs: vec![],
    });

    stories
}

pub fn run(ps2: &mut Ps2, fb_vaddr: u64) {
    if fb_vaddr == 0 { return; }

    let mut stories = make_stories();
    let mut current: usize = 0;
    let mut active_knob: usize = 0;

    loop {
        let total = stories.len();
        let source = stories[current].render_source();
        let title = stories[current].title;
        let n_knobs = stories[current].knobs.len();

        let mut fb = Framebuffer::new(640, 480);
        fb.clear(Color::DARK_BG);

        // Header
        fb.fill_rect(0, 0, 640, 28, Color::rgb(25, 25, 30));
        fb.draw_text(8, 8, "PST OS Storybook", Color::rgb(59, 130, 246), Color::rgb(25, 25, 30));
        let counter = format!("{}/{}", current + 1, total);
        fb.draw_text(640 - counter.len() * 8 - 8, 8, &counter, Color::rgb(120, 120, 120), Color::rgb(25, 25, 30));

        // Title bar with section tag
        fb.fill_rect(0, 28, 640, 22, Color::rgb(35, 35, 40));
        let is_combo = title.starts_with("Combo:");
        if is_combo {
            fb.fill_rect(6, 31, 52, 16, Color::rgb(234, 88, 12));
            fb.draw_text(10, 33, "COMBO", Color::WHITE, Color::rgb(234, 88, 12));
            fb.draw_text(64, 33, &title[7..], Color::WHITE, Color::rgb(35, 35, 40));
        } else {
            fb.fill_rect(6, 31, 40, 16, Color::rgb(59, 130, 246));
            fb.draw_text(10, 33, "UNIT", Color::WHITE, Color::rgb(59, 130, 246));
            fb.draw_text(52, 33, title, Color::WHITE, Color::rgb(35, 35, 40));
        }
        fb.draw_hline(0, 50, 640, if is_combo { Color::rgb(234, 88, 12) } else { Color::rgb(59, 130, 246) });

        // Knob panel (right side, only for unit stories)
        let has_knobs = n_knobs > 0;
        let knob_x: usize = 420;
        let knob_w: usize = 220;
        let content_w: usize = if has_knobs { 400 } else { 620 };
        if has_knobs {
        fb.fill_rect(knob_x - 2, 52, knob_w + 2, 260, Color::rgb(28, 28, 33));
        fb.draw_text(knob_x + 4, 56, "Properties", Color::rgb(59, 130, 246), Color::rgb(28, 28, 33));
        fb.draw_hline(knob_x, 68, knob_w - 4, Color::rgb(50, 50, 55));

        let mut ky: usize = 74;
        for (i, knob) in stories[current].knobs.iter().enumerate() {
            let is_active = i == active_knob;
            let bg = if is_active { Color::rgb(45, 45, 55) } else { Color::rgb(28, 28, 33) };
            let name_color = if is_active { Color::rgb(59, 130, 246) } else { Color::rgb(140, 140, 140) };

            fb.fill_rect(knob_x, ky, knob_w - 4, 28, bg);

            // Knob number
            let num = format!("{}.", i + 1);
            fb.draw_text(knob_x + 4, ky + 2, &num, Color::rgb(80, 80, 80), bg);

            // Knob name
            fb.draw_text(knob_x + 24, ky + 2, knob.name, name_color, bg);

            // Current value
            let val_display = if knob.val().is_empty() { "(default)" } else { knob.val() };
            let val_color = if is_active { Color::WHITE } else { Color::rgb(180, 180, 180) };
            fb.draw_text(knob_x + 24, ky + 14, val_display, val_color, bg);

            // Arrow indicator for active
            if is_active {
                fb.draw_text(knob_x + knob_w - 28, ky + 8, ">>", Color::rgb(59, 130, 246), bg);
            }

            ky += 30;
        }

        // Instructions below knobs
        let inst_y = ky + 8;
        fb.draw_text(knob_x + 4, inst_y, "Up/Down = knob", Color::rgb(80, 80, 80), Color::rgb(28, 28, 33));
        fb.draw_text(knob_x + 4, inst_y + 12, "Enter  = cycle", Color::rgb(80, 80, 80), Color::rgb(28, 28, 33));
        fb.draw_text(knob_x + 4, inst_y + 24, "1-9    = knob #", Color::rgb(80, 80, 80), Color::rgb(28, 28, 33));
        } // end has_knobs

        // Render Markout content
        let content_h: usize = 250;
        let mut content_fb = Framebuffer::new(content_w, content_h);
        content_fb.clear(Color::DARK_BG);
        render_markout(&mut content_fb, &source, Color::DARK_BG, Color::WHITE);

        for cy in 0..content_h {
            for cx in 0..content_w {
                let idx = (cy * content_w + cx) * 4;
                if idx + 3 < content_fb.pixels.len() {
                    let b = content_fb.pixels[idx];
                    let g = content_fb.pixels[idx + 1];
                    let r = content_fb.pixels[idx + 2];
                    fb.set_pixel(cx + 8, 54 + cy, Color::rgb(r, g, b));
                }
            }
        }

        // Source display (bottom)
        let src_y: usize = 310;
        fb.fill_rect(0, src_y, 640, 140, Color::rgb(20, 20, 25));
        fb.draw_text(8, src_y + 4, "Markout Source:", Color::rgb(80, 80, 80), Color::rgb(20, 20, 25));
        fb.draw_hline(0, src_y + 16, 640, Color::rgb(40, 40, 45));
        let mut sy = src_y + 20;
        for line in source.lines() {
            if sy + 12 > 446 { break; }
            let display = if line.len() > 78 { &line[..78] } else { line };
            fb.draw_text(8, sy, display, Color::rgb(130, 180, 130), Color::rgb(20, 20, 25));
            sy += 12;
        }

        // Navigation bar
        fb.fill_rect(0, 452, 640, 28, Color::rgb(30, 30, 35));
        fb.fill_rect(4, 456, 64, 20, Color::rgb(50, 50, 55));
        fb.draw_text(12, 460, "<- Prev", Color::WHITE, Color::rgb(50, 50, 55));
        fb.draw_text(200, 462, "P=Snap All", Color::rgb(234, 88, 12), Color::rgb(30, 30, 35));
        fb.draw_text(350, 462, "Esc=close", Color::rgb(70, 70, 70), Color::rgb(30, 30, 35));
        fb.fill_rect(572, 456, 64, 20, Color::rgb(50, 50, 55));
        fb.draw_text(580, 460, "Next ->", Color::WHITE, Color::rgb(50, 50, 55));

        // Blit
        let vga = fb_vaddr as *mut u8;
        unsafe { core::ptr::copy_nonoverlapping(fb.pixels.as_ptr(), vga, 640 * 4 * 480); }

        // Input
        match ps2.read_event() {
            InputEvent::Key(0x1B) => return,
            // Previous story
            InputEvent::Key(b'[') => {
                if current > 0 { current -= 1; } else { current = total - 1; }
                active_knob = 0;
            }
            // Next story
            InputEvent::Key(b']') | InputEvent::Key(b' ') => {
                current = (current + 1) % total;
                active_knob = 0;
            }
            // Knob navigation: up/down arrows (scan codes mapped to W/S as fallback)
            InputEvent::Key(0x48) | InputEvent::Key(b'w') => {
                if n_knobs > 0 {
                    if active_knob > 0 { active_knob -= 1; }
                    else { active_knob = n_knobs - 1; }
                }
            }
            InputEvent::Key(0x50) | InputEvent::Key(b's') => {
                if n_knobs > 0 { active_knob = (active_knob + 1) % n_knobs; }
            }
            // Cycle active knob
            InputEvent::Key(b'\n') | InputEvent::Key(b'd') | InputEvent::Key(0x4D) => {
                if n_knobs > 0 { stories[current].knobs[active_knob].cycle(); }
            }
            // Cycle backwards
            InputEvent::Key(b'a') | InputEvent::Key(0x4B) => {
                if n_knobs > 0 {
                    let knob = &mut stories[current].knobs[active_knob];
                    if knob.selected > 0 { knob.selected -= 1; }
                    else { knob.selected = knob.options.len() - 1; }
                }
            }
            // Number keys select knob directly and cycle it
            InputEvent::Key(ch) if ch >= b'1' && ch <= b'9' => {
                let idx = (ch - b'1') as usize;
                if idx < n_knobs {
                    active_knob = idx;
                    stories[current].knobs[idx].cycle();
                }
            }
            // Click handling
            InputEvent::Click { x, y } => {
                if y >= 452 {
                    if x < 80 {
                        if current > 0 { current -= 1; } else { current = total - 1; }
                        active_knob = 0;
                    } else if x >= 560 {
                        current = (current + 1) % total;
                        active_knob = 0;
                    }
                } else if x >= 420 && y >= 74 {
                    // Click on knob panel
                    let knob_idx = (y - 74) / 30;
                    if knob_idx < n_knobs {
                        active_knob = knob_idx;
                        stories[current].knobs[knob_idx].cycle();
                    }
                }
            }
            // Snap All: auto-ride through every story, cycling styles
            InputEvent::Key(b'p') => {
                snap_all(&mut stories, fb_vaddr);
            }
            _ => {}
        }
    }
}

fn snap_all(stories: &mut Vec<Story>, fb_vaddr: u64) {
    serial_print("<<SNAP_BEGIN>>\n");

    let total = stories.len();
    let mut snap_count: usize = 0;

    for si in 0..total {
        let has_style_knob = stories[si].knobs.iter()
            .position(|k| k.name == "style");

        if let Some(style_idx) = has_style_knob {
            let n_options = stories[si].knobs[style_idx].options.len();
            for opt in 0..n_options {
                stories[si].knobs[style_idx].selected = opt;
                let style_name = stories[si].knobs[style_idx].val();
                let snap_name = if style_name.is_empty() {
                    format!("{}_default", stories[si].title)
                } else {
                    format!("{}_{}", stories[si].title, style_name)
                };

                render_snap(stories, si, fb_vaddr);
                snap_count += 1;

                serial_print("<<SNAP:");
                serial_print(&snap_name);
                serial_print(">>\n");

                snap_delay();
            }
            // Reset to default
            stories[si].knobs[style_idx].selected = 0;
        } else {
            // Combo or no style knob — snap once
            render_snap(stories, si, fb_vaddr);
            snap_count += 1;

            serial_print("<<SNAP:");
            serial_print(stories[si].title);
            serial_print(">>\n");

            snap_delay();
        }
    }

    let mut buf = [0u8; 8];
    serial_print("<<SNAP_END:");
    serial_print(fmt_num(snap_count, &mut buf));
    serial_print(">>\n");
}

fn rdtsc() -> u64 {
    let lo: u32;
    let hi: u32;
    unsafe { core::arch::asm!("rdtsc", out("eax") lo, out("edx") hi, options(nomem, nostack)); }
    ((hi as u64) << 32) | lo as u64
}

fn snap_delay() {
    let start = rdtsc();
    while rdtsc() - start < 1_000_000_000 {}
}

fn fmt_num(n: usize, buf: &mut [u8; 8]) -> &str {
    let mut i = buf.len();
    let mut v = n;
    if v == 0 {
        i -= 1;
        buf[i] = b'0';
    } else {
        while v > 0 && i > 0 {
            i -= 1;
            buf[i] = b'0' + (v % 10) as u8;
            v /= 10;
        }
    }
    core::str::from_utf8(&buf[i..]).unwrap_or("?")
}

fn render_snap(stories: &Vec<Story>, idx: usize, fb_vaddr: u64) {
    let source = stories[idx].render_source();
    let title = stories[idx].title;
    let is_combo = title.starts_with("Combo:");

    let mut fb = Framebuffer::new(640, 480);
    fb.clear(Color::DARK_BG);

    // Header
    fb.fill_rect(0, 0, 640, 28, Color::rgb(25, 25, 30));
    fb.draw_text(8, 8, "PST OS Storybook", Color::rgb(59, 130, 246), Color::rgb(25, 25, 30));

    // Title with section tag
    fb.fill_rect(0, 28, 640, 22, Color::rgb(35, 35, 40));
    if is_combo {
        fb.fill_rect(6, 31, 52, 16, Color::rgb(234, 88, 12));
        fb.draw_text(10, 33, "COMBO", Color::WHITE, Color::rgb(234, 88, 12));
        fb.draw_text(64, 33, &title[7..], Color::WHITE, Color::rgb(35, 35, 40));
    } else {
        fb.fill_rect(6, 31, 40, 16, Color::rgb(59, 130, 246));
        fb.draw_text(10, 33, "UNIT", Color::WHITE, Color::rgb(59, 130, 246));
        fb.draw_text(52, 33, title, Color::WHITE, Color::rgb(35, 35, 40));
    }
    fb.draw_hline(0, 50, 640, if is_combo { Color::rgb(234, 88, 12) } else { Color::rgb(59, 130, 246) });

    // Show current knob values in top-right
    let n_knobs = stories[idx].knobs.len();
    if n_knobs > 0 {
        let mut tag = String::new();
        for knob in &stories[idx].knobs {
            if !tag.is_empty() { tag.push_str("  "); }
            tag.push_str(knob.name);
            tag.push('=');
            let v = knob.val();
            tag.push_str(if v.is_empty() { "default" } else { v });
        }
        let tx = 640usize.saturating_sub(tag.len() * 8 + 8);
        fb.draw_text(tx, 33, &tag, Color::rgb(120, 120, 120), Color::rgb(35, 35, 40));
    }

    // Render content — full width for snaps
    let content_w: usize = 620;
    let content_h: usize = 400;
    let mut content_fb = Framebuffer::new(content_w, content_h);
    content_fb.clear(Color::DARK_BG);
    render_markout(&mut content_fb, &source, Color::DARK_BG, Color::WHITE);

    for cy in 0..content_h {
        for cx in 0..content_w {
            let pi = (cy * content_w + cx) * 4;
            if pi + 3 < content_fb.pixels.len() {
                let b = content_fb.pixels[pi];
                let g = content_fb.pixels[pi + 1];
                let r = content_fb.pixels[pi + 2];
                fb.set_pixel(cx + 10, 54 + cy, Color::rgb(r, g, b));
            }
        }
    }

    // Snap indicator
    fb.fill_rect(0, 458, 640, 22, Color::rgb(234, 88, 12));
    fb.draw_text(240, 462, "SNAP", Color::WHITE, Color::rgb(234, 88, 12));

    // Blit
    let vga = fb_vaddr as *mut u8;
    unsafe { core::ptr::copy_nonoverlapping(fb.pixels.as_ptr(), vga, 640 * 4 * 480); }
}
