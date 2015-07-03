use std::cmp::max;

use libc::iscntrl;
use x11::xlib;
use x11::keysym;
use xmini::{Display, Window, GC, XFontStruct, Color, KeyPressedEvent};

pub struct UI {
    x: i32,
    y: i32,
    w: u32,
    h: u32,
    display: Display,
    window: Window,
    gc: GC,
    xfont: XFontStruct,
    colfg: Color,
    colbg: Color,
    selcolfg: Color,
    selcolbg: Color,
    cursor: usize,
}

impl UI {
    pub fn new(settings: &super::Settings) -> UI {
        let display = Display::new();

        display.wait_keyboard();

        let xfont = display.new_font(&settings.font);

        let screen = display.get_default_screen();
        let root = screen.get_root_window();
        let cmap = screen.get_default_colormap();

        let color_fg = display.alloc_named_color(&cmap, &settings.normfgcolor);
        let color_bg = display.alloc_named_color(&cmap, &settings.normbgcolor);
        let sel_color_fg = display.alloc_named_color(&cmap, &settings.selfgcolor);
        let sel_color_bg = display.alloc_named_color(&cmap, &settings.selbgcolor);

        let (width, height) = screen.get_geometry(&xfont);

        let window = root.new_child_window(width, height, &color_bg);

        display.wait_until_map_notify();

        UI {
            x: 0,
            y: 0,
            w: width,
            h: height,
            display: display,
            gc: window.new_gc(color_fg.clone(), color_bg.clone()),
            window: window,
            xfont: xfont,
            colfg: color_fg.clone(),
            colbg: color_bg.clone(),
            selcolfg: sel_color_fg.clone(),
            selcolbg: sel_color_bg.clone(),
            cursor: 0,
        }
    }

    fn get_items_page(&self, status: &super::Status) -> (Vec<String>, u32) {
        let mut current_page = 0;

        // Calculate the space for the words
        let max_item_length = status.items.iter().fold(0, |acc, item| max(acc, item.len()));
        let input_width = self.xfont.text_width(&"_".to_string()) as i32 * max_item_length as i32;
        let mut words_width = self.w as i32;
        words_width -= 2;
        words_width -= self.xfont.text_width(&status.settings.prompt) as i32 + 4;
        words_width -= input_width + 8;
        words_width -= self.xfont.text_width(&"<".to_string()) as i32 + 4;
        words_width -= self.xfont.text_width(&">".to_string()) as i32 - 7;

        let mut page_items = vec![];
        let mut current_x_pos = 0;
        for item in &status.matches {
            let item_width = (self.xfont.text_width(&item) + 10) as i32;
            if current_x_pos + item_width > words_width {
                current_page += 1;
                current_x_pos = item_width;
            } else {
                current_x_pos += item_width;
                if current_page == status.page {
                    page_items.push(item.clone());
                }
            }
        }
        (page_items, current_page + 1)
    }

    fn draw_bg(&self, x: i32, y: i32, w: u32, h: u32, selected: bool) {
        if selected {
            self.gc.set_background(&self.selcolfg);
            self.gc.set_foreground(&self.selcolbg);
        } else {
            self.gc.set_background(&self.colfg);
            self.gc.set_foreground(&self.colbg);
        }

        self.gc.fill_rectangle(self.x + x, self.y + y, w, h);
        self.display.flush();
    }

    fn draw_rect(&self, x: i32, y: i32, w: u32, h: u32, fill: bool, selected: bool) {
        if selected {
            self.gc.set_foreground(&self.selcolfg);
            self.gc.set_background(&self.selcolbg);
        } else {
            self.gc.set_foreground(&self.colfg);
            self.gc.set_background(&self.colbg);
        }

        if fill {
            self.gc.fill_rectangle(self.x + x, self.y + y, w, h);
        } else {
            self.gc.draw_rectangle(self.x + x, self.y + y, w-1, h-1);
        }
        self.display.flush();
    }

    fn draw_text(&self, x: i32, y: i32, padding: u32, text: &String, selected: bool) {
        let width = self.xfont.text_width(text);
        let height = self.xfont.text_height() as i32;
        self.draw_bg(x, y - height, width + padding, y as u32 + 5, selected);

        if selected {
            self.gc.set_foreground(&self.selcolfg);
            self.gc.set_background(&self.selcolbg);
        } else {
            self.gc.set_foreground(&self.colfg);
            self.gc.set_background(&self.colbg);
        }
        self.gc.set_font(&self.xfont);
        self.gc.draw_string(x + padding as i32, y, text);
        self.display.flush();
    }

    fn draw_horizontal_items(&self, x: i32, status: &super::Status) -> i32 {
        let mut x_pos = x;

        // Draw prev icon
        if status.page > 0 {
            self.draw_text(x_pos, self.xfont.font_height() as i32, 5, &"<".to_string(), false);
            x_pos += self.xfont.text_width(&"<".to_string()) as i32 + 4;
        }

        // Draw horizontal matches
        let (match_items, pages) = self.get_items_page(&status);

        if pages > status.page + 1 {
            // Draw next icon and break
            self.draw_text(self.w as i32 - self.xfont.text_width(&">".to_string()) as i32 - 5, self.xfont.font_height() as i32, 5, &">".to_string(), false);
        }

        for match_item in match_items {
            self.draw_text(x_pos, self.xfont.font_height() as i32, 5, &match_item, *match_item == status.selected);
            x_pos += (self.xfont.text_width(&match_item) + 10) as i32;
        }
        x_pos
    }

    fn draw_vertical_items(&self, x: i32, status: &super::Status) -> i32 {
        unimplemented!();
    }

    fn draw_prompt(&self, x: i32, status: &super::Status) -> i32 {
        if status.settings.prompt != "" {
            self.draw_text(x, self.xfont.font_height() as i32, 5, &status.settings.prompt, false);
            x + self.xfont.text_width(&status.settings.prompt) as i32 + 4
        } else { x }
    }

    fn draw_input(&self, x: i32, status: &super::Status) -> i32 {
        let max_item_length = status.items.iter().fold(0, |acc, item| max(acc, item.len()));
        let input_width = self.xfont.text_width(&"_".to_string()) * max_item_length as u32;

        self.draw_text(x, self.xfont.font_height() as i32, 0, &status.text, false);

        // Draw cursor
        self.gc.set_foreground(&self.colfg);
        self.gc.set_background(&self.colbg);
        self.draw_rect(
            self.x + x + (self.xfont.text_width(&status.text[0..self.cursor].to_string()) as i32),
            self.y + 4,
            0,
            self.xfont.font_height() - 3,
            false,
            false
        );
        self.display.flush();

        x + (input_width + 8) as i32
	}


    pub fn draw_menu(&self, status: &super::Status) {
		let mut x_pos = 2;
        self.draw_bg(0, 0, self.w, self.h, false);

        x_pos = self.draw_prompt(x_pos, &status);
        x_pos = self.draw_input(x_pos, &status);

        if status.settings.lines > 0 {
            // Draw vertical matches
            // TODO
            self.draw_vertical_items(x_pos, &status);
        } else {
			self.draw_horizontal_items(x_pos, &status);
		}
    }

    fn translate_keypress(&mut self, event_state: u32, ksym: u32) -> (u32, u32) {
        if event_state & xlib::ControlMask != 0 {
            match ksym {
                keysym::XK_a => (xlib::ControlMask, keysym::XK_Home),
                keysym::XK_b => (xlib::ControlMask, keysym::XK_Left),
                keysym::XK_c => (xlib::ControlMask, keysym::XK_Escape),
                keysym::XK_d => (xlib::ControlMask, keysym::XK_Delete),
                keysym::XK_e => (xlib::ControlMask, keysym::XK_End),
                keysym::XK_f => (xlib::ControlMask, keysym::XK_Right),
                keysym::XK_h => (xlib::ControlMask, keysym::XK_BackSpace),
                keysym::XK_i => (xlib::ControlMask, keysym::XK_Tab),
                keysym::XK_j => (xlib::ControlMask, keysym::XK_Return),
                keysym::XK_m => (xlib::ControlMask, keysym::XK_Return),
                keysym::XK_n => (xlib::ControlMask, keysym::XK_Down),
                keysym::XK_p => (xlib::ControlMask, keysym::XK_Up),
                _ => (xlib::ControlMask, ksym)
            }
        } else if event_state & xlib::Mod1Mask != 0 {
            match ksym  {
                keysym::XK_g => (xlib::Mod1Mask, keysym::XK_Home),
                keysym::XK_G => (xlib::Mod1Mask, keysym::XK_End),
                keysym::XK_h => (xlib::Mod1Mask, keysym::XK_Up),
                keysym::XK_j => (xlib::Mod1Mask, keysym::XK_Next),
                keysym::XK_k => (xlib::Mod1Mask, keysym::XK_Prior),
                keysym::XK_l => (xlib::Mod1Mask, keysym::XK_Down),
                _ => (xlib::Mod1Mask, ksym)
            }
        } else {
            (0, ksym)
        }
    }

    fn keypress(&mut self, event: &mut KeyPressedEvent, status: &mut super::Status) -> bool {
        let old_text = status.text.clone();

        let ksym = self.translate_keypress(event.state(), event.lookup_keysym());
        let input = event.lookup_string();

        match ksym {
            (xlib::ControlMask, keysym::XK_k) => { status.text.remove(self.cursor); ()},
            (xlib::ControlMask, keysym::XK_u) => { status.text.remove(self.cursor); ()},
            (xlib::ControlMask, keysym::XK_w) => {
                    while self.cursor > 0 && status.text.remove(self.cursor) == ' ' { self.cursor -= 1; }
                    while self.cursor > 0 && status.text.remove(self.cursor) != ' ' { self.cursor -= 1; }
                },
            // TODO: Understand and implement it
            // (xlib::ControlMask, keysym::XK_y) => {
            //     unsafe {
            //         xlib::XConvertSelection(self.display, if (event.state() & xlib::ShiftMask != 0) { clip } else { xlib::XA_PRIMARY }, utf8, utf8, win, xlib::CurrentTime);
            //     }
            //     return false;
            // },
            (_, keysym::XK_Delete) => {
                if status.text.len() >= self.cursor {
                    status.text.remove(self.cursor);
                }
            },
            (_, keysym::XK_BackSpace) => {
                if self.cursor > 0 && status.text.len() > 0 {
                    status.text.remove(self.cursor-1);
                    self.cursor -= 1;
                }
            },
            (_, keysym::XK_End) => {
                if self.cursor < status.text.len() {
                    self.cursor = status.text.len();
                } else {
                    status.selected = status.matches.last().unwrap_or(&"".to_string()).clone();
                }
            },
            (_, keysym::XK_Escape) => return true,
            (_, keysym::XK_Home) => {
                if status.selected == status.matches.first().unwrap_or(&"".to_string()).clone() {
                    self.cursor = 0;
                } else {
                    status.selected = status.matches.first().unwrap_or(&"".to_string()).clone();
                }
            },
            (_, keysym::XK_Left) => {
                if status.selected == status.matches.first().unwrap_or(&"".to_string()).clone() {
                    if self.cursor > 0 {
                        self.cursor -= 1;
                    }
                } else if status.settings.lines == 0 {
                    status.selected = match status.matches.binary_search(&status.selected) {
                        Ok(n) => status.matches[n - 1].clone(),
                        Err(_) => "".to_string()
                    }
                }
            },
            (_, keysym::XK_Up) => {
                match status.matches.binary_search(&status.selected) {
                    Ok(0) => return false,
                    Ok(n) => status.selected = status.matches[n - 1].clone(),
                    Err(_) => return false
                }
            },
            (_, keysym::XK_Next) => {
                // TODO: Calc the number of pages
                status.page += 1
            },
            (_, keysym::XK_Prior) => {
                if status.page > 0 {
                    status.page -= 1
                }
            },
            (_, keysym::XK_Return) => {
                if (event.state() & xlib::ShiftMask) != 0 || status.selected == "" {
                    println!("{}", status.text)
                } else {
                    println!("{}", status.selected)
                }
                return true
            },
            (_, keysym::XK_KP_Enter) => {
                if (event.state() & xlib::ShiftMask) != 0 || status.selected == "" {
                    println!("{}", status.text)
                } else {
                    println!("{}", status.selected)
                }
                return true
            },
            (_, keysym::XK_Right) => {
                if self.cursor < status.text.len() {
                    self.cursor += 1;
                } else  {
                    match status.matches.binary_search(&status.selected) {
                        Ok(n) => {
                            if n < (status.matches.len() - 1) {
                                status.selected = status.matches[n + 1].clone();
                            } else {
                                return false
                            }
                        },
                        Err(_) => return false
                    }
                }
            },
            (_, keysym::XK_Down) => {
                match status.matches.binary_search(&status.selected) {
                    Ok(n) => {
                        if n < (status.matches.len() - 1) {
                            status.selected = status.matches[n + 1].clone();
                        } else {
                            return false
                        }
                    },
                    Err(_) => return false
                }
            },
            (_, keysym::XK_Tab) => {
                if status.selected != "" {
                    status.text = status.selected.clone();
                    self.cursor = status.text.len();
                }
            },
            (_, _) => unsafe {
                if iscntrl(input.chars().nth(0).unwrap_or(0 as char) as i32) == 0 {
                    status.text.insert(self.cursor, input.chars().nth(0).unwrap());
                    self.cursor += 1;
                }
            },
        }
        if old_text != status.text {
            if status.settings.matcher == "fuzzy" {
                status.matches = super::matches::fuzzy_match(&status.text, &status.items);
            } else if status.settings.matcher == "dmenu" {
                status.matches = super::matches::dmenu_match(&status.text, &status.items);
            } else {
                status.matches = super::matches::simple_match(&status.text, &status.items);
            }
            status.page = 0;
            if !status.matches.contains(&status.selected) {
                status.selected = status.matches.first().unwrap_or(&"".to_string()).clone()
            }
        }
        self.draw_menu(&status);
        return false
    }

    fn paste(&self) {
        panic!("Not implemented");
    }

    pub fn run(&mut self, mut status: super::Status) {
        self.draw_menu(&status);

        loop {
            match self.display.next_event() {
                Some(mut event) => {
                    if event.filter_event(&mut self.window) { continue; }
                    match event.get_type() {
                        xlib::Expose => {
                            if event.to_expose_event().count() == 0 {
                                self.draw_menu(&status);
                            }
                        },
                        xlib::KeyPress => {
                            if self.keypress(&mut event.to_keypress_event(), &mut status) { break }
                        },
                        xlib::SelectionNotify => {
                            // if event.to_selection_event().property() == utf8 { <-- This variable
                            // utf8 must be obtained in any way, review the dmenu code
                            //     self.paste();
                            // }
                            if event.to_selection_event().property() == 0 {
                                self.paste();
                            }
                        },
                        xlib::VisibilityNotify => if event.to_visibility_event().state() != xlib::VisibilityUnobscured {
                            self.window.raise()
                        },
                        _ => continue
                    }
                },
                None => break
            }
        }
    }
}
