use std::ffi::CString;
use std::mem::zeroed;
use std::cmp::max;
use std::str::from_utf8;
use std::convert::From;
use std::thread::sleep_ms;
use std::env;
use std::ptr::{
  null_mut,
  read,
};

use libc::c_uint;
use libc::c_int;
use libc::exit;
use libc::funcs::c95::ctype::iscntrl;
use x11::xlib;
use x11::keysym;

pub struct UI {
    x: i32,
    y: i32,
    w: u32,
    h: u32,
    display: *mut xlib::Display,
    window: xlib::Window,
    gc: xlib::GC,
    xfont: *mut xlib::XFontStruct,
    colfg: u64,
    colbg: u64,
    selcolfg: u64,
    selcolbg: u64,
    cursor: usize,
}

impl UI {
    unsafe fn setup_keyboard(display: *mut xlib::Display) {
        /* try to grab keyboard, we may have to wait for another process to ungrab */
        for _ in 1..1000 {
            if xlib::XGrabKeyboard(display, xlib::XDefaultRootWindow(display), 1, xlib::GrabModeAsync, xlib::GrabModeAsync, xlib::CurrentTime) == xlib::GrabSuccess {
                return
            }
            sleep_ms(1000);
        }
        panic!("cannot grab keyboard");
    }

    unsafe fn setup_display() -> *mut xlib::Display {
        let mut display_env;

        match env::var("DISPLAY") {
            Ok(val) => display_env = CString::new(val).unwrap(),
            Err(_) => display_env = CString::new("").unwrap(),
        }

        let display = xlib::XOpenDisplay(display_env.as_ptr());

        if display == null_mut() {
            panic!("Cannot connect to X Server: {}", from_utf8(display_env.as_bytes()).unwrap());
        }
        display
    }

    unsafe fn setup_colors(display: *mut xlib::Display, screen: i32, settings: &super::Settings) -> (u64, u64, u64, u64) {
        let cmap = xlib::XDefaultColormap(display, screen);

        let mut color_fg: xlib::XColor = zeroed();
        xlib::XAllocNamedColor(display, cmap, CString::new(settings.normfgcolor.clone()).unwrap().as_ptr(), &mut color_fg, &mut color_fg);
        let mut color_bg: xlib::XColor = zeroed();
        xlib::XAllocNamedColor(display, cmap, CString::new(settings.normbgcolor.clone()).unwrap().as_ptr(), &mut color_bg, &mut color_bg);

        let mut sel_color_fg: xlib::XColor = zeroed();
        xlib::XAllocNamedColor(display, cmap, CString::new(settings.selfgcolor.clone()).unwrap().as_ptr(), &mut sel_color_fg, &mut sel_color_fg);
        let mut sel_color_bg: xlib::XColor = zeroed();
        xlib::XAllocNamedColor(display, cmap, CString::new(settings.selbgcolor.clone()).unwrap().as_ptr(), &mut sel_color_bg, &mut sel_color_bg);

        (color_fg.pixel, color_bg.pixel, sel_color_fg.pixel, sel_color_bg.pixel)
    }

    unsafe fn setup_window(display: *mut xlib::Display, root: u64, width: u32, height: u32, color_bg: u64) -> xlib::Window {
        let mut attributes: xlib::XSetWindowAttributes = zeroed();
        attributes.background_pixel = color_bg;
        attributes.override_redirect = 1;
        attributes.event_mask =  xlib::StructureNotifyMask | xlib::ExposureMask | xlib::KeyPressMask | xlib::VisibilityChangeMask;


        let window = xlib::XCreateWindow(display, root, 0, 0, width, height, 0, 0,
                                         xlib::InputOutput as c_uint, null_mut(),
                                         xlib::CWOverrideRedirect | xlib::CWBackPixel | xlib::CWEventMask, &mut attributes);
        // Show window
        xlib::XMapWindow(display, window);
        xlib::XFlush(display);
        window
    }

    unsafe fn setup_gc(display: *mut xlib::Display, window: xlib::Window, color_fg: u64, color_bg: u64) -> xlib::GC {
        let mut values: xlib::XGCValues = zeroed();
        let valuesmask: u64 = 0 as u64;

        let gc = xlib::XCreateGC(display, window, valuesmask, &mut values);
        xlib::XSetForeground(display, gc, color_fg);
        xlib::XSetBackground(display, gc, color_bg);

        xlib::XSetLineAttributes(display, gc, 1, xlib::LineSolid, xlib::CapButt, xlib::JoinMiter);
        xlib::XSetFillStyle(display, gc, xlib::FillSolid);

        xlib::XSync(display, 1);
        xlib::XFlush(display);
        gc
    }

    unsafe fn setup_font(display: *mut xlib::Display, font_name: &String) -> *mut xlib::XFontStruct {
        let fontstr = CString::new(font_name.clone()).unwrap();
        xlib::XLoadQueryFont(display, fontstr.as_ptr())
    }

    unsafe fn wait_until_map_notify(display: *mut xlib::Display) {
        loop {
            let mut e = zeroed();
            xlib::XNextEvent(display, &mut e);
            if e.get_type() == xlib::MapNotify {
                break;
            }
        }
    }

    unsafe fn get_screen(display: *mut xlib::Display) -> i32 {
        xlib::XDefaultScreen(display)
    }

    unsafe fn get_root_window(display: *mut xlib::Display, screen: i32) -> xlib::Window {
        xlib::XRootWindow(display, screen)
    }

    unsafe fn get_geometry(display: *mut xlib::Display, screen: i32, xfont: *mut xlib::XFontStruct) -> (u32, u32) {
        let width = xlib::XDisplayWidth(display, screen) as u32;
        let height = (read(xfont).max_bounds.ascent + read(xfont).max_bounds.descent + 4) as u32;
        (width, height)
    }

    pub fn new(settings: &super::Settings) -> UI {
        unsafe {
            let display = UI::setup_display();
            UI::setup_keyboard(display);
            let screen = UI::get_screen(display);
            let root = UI::get_root_window(display, screen);

            let (color_fg, color_bg, sel_color_fg, sel_color_bg) = UI::setup_colors(display, screen, &settings);

            let xfont = UI::setup_font(display, &settings.font);

            let (width, height) = UI::get_geometry(display, screen, xfont);

            let window = UI::setup_window(display, root, width, height, color_bg);

            UI::wait_until_map_notify(display);

            let gc = UI::setup_gc(display, window, color_fg, color_bg);
            UI {
                x: 0,
                y: 0,
                w: width,
                h: height,
                display: display,
                window: window,
                gc: gc,
                xfont: xfont,
                colfg: color_fg,
                colbg: color_bg,
                selcolfg: sel_color_fg,
                selcolbg: sel_color_bg,
                cursor: 0,
            }
        }
    }

    unsafe fn get_items_page(&self, status: &super::Status, page: u32) -> (Vec<String>, u32) {
        let mut current_page = 0;

        // Calculate the space for the words
        let max_item_length = status.items.iter().fold(0, |acc, item| max(acc, item.len()));
        let input_width = self.text_width(&"_".to_string()) as i32 * max_item_length as i32;
        let mut words_width = self.w as i32;
        words_width -= 2;
        words_width -= self.text_width(&status.settings.prompt) as i32 + 4;
        words_width -= input_width + 8;
        words_width -= self.text_width(&"<".to_string()) as i32 + 4;
        words_width -= self.text_width(&">".to_string()) as i32 - 7;

        let mut page_items = vec![];
        let mut current_x_pos = 0;
        for item in &status.matches {
            let item_width = (self.text_width(&item) + 10) as i32;
            if current_x_pos + item_width > words_width {
                current_page += 1;
                current_x_pos = item_width;
            } else {
                current_x_pos += item_width;
                if current_page == page {
                    page_items.push(item.clone());
                }
            }
        }
        (page_items, current_page + 1)
    }

    unsafe fn draw_bg(&self, x: i32, y: i32, w: u32, h: u32, selected: bool) {
        if selected {
            xlib::XSetForeground(self.display, self.gc, self.selcolbg);
            xlib::XSetBackground(self.display, self.gc, self.selcolfg);
        } else {
            xlib::XSetForeground(self.display, self.gc, self.colbg);
            xlib::XSetBackground(self.display, self.gc, self.colfg);
        }

        xlib::XFillRectangle(self.display, self.window, self.gc, self.x + x, self.y + y, w, h);
        xlib::XFlush(self.display);
    }

    unsafe fn draw_text(&self, x: i32, y: i32, padding: u32, text: &String, selected: bool) {
        let width = self.text_width(text);
        let height = self.text_height() as i32;
        self.draw_bg(x, y - height, width + padding, y as u32 + 5, selected);

        if selected {
            xlib::XSetForeground(self.display, self.gc, self.selcolfg);
            xlib::XSetBackground(self.display, self.gc, self.selcolbg);
        } else {
            xlib::XSetForeground(self.display, self.gc, self.colfg);
            xlib::XSetBackground(self.display, self.gc, self.colbg);
        }
        xlib::XSetFont(self.display, self.gc, read(self.xfont).fid);
        xlib::XDrawString(self.display, self.window, self.gc, x + padding as i32, y as i32, CString::new(text.clone()).unwrap().as_ptr(), text.len() as i32);
        xlib::XFlush(self.display);
    }

    unsafe fn text_width(&self, text: &String) -> u32 {
        let font_width = read(self.xfont).max_bounds.rbearing - read(self.xfont).min_bounds.lbearing;
        (text.len() * font_width as usize) as u32
    }

    unsafe fn text_height(&self) -> u32 {
        (read(self.xfont).max_bounds.ascent + read(self.xfont).max_bounds.descent) as u32
    }

    pub fn draw_menu(&self, status: &super::Status) {
        unsafe {
            self.draw_bg(0, 0, self.w, self.h, false);
            let max_item_length = status.items.iter().fold(0, |acc, item| max(acc, item.len()));

            let input_width = self.text_width(&"_".to_string()) * max_item_length as u32;
            let font_height = self.text_height();
            let mut current_x_pos = 2;

            // Draw Prompt
            if status.settings.prompt != "" {
                self.draw_text(current_x_pos, font_height as i32, 5, &status.settings.prompt, false);
                current_x_pos += self.text_width(&status.settings.prompt) as i32 + 4;
            }

            // Draw input
            self.draw_text(current_x_pos, font_height as i32, 0, &status.text, false);

            // Draw cursor
            xlib::XSetForeground(self.display, self.gc, self.colfg);
            xlib::XSetBackground(self.display, self.gc, self.colbg);
            xlib::XDrawRectangle(
                self.display,
                self.window,
                self.gc,
                self.x + current_x_pos + (self.text_width(&status.text[0..self.cursor].to_string()) as i32),
                self.y + 4, 0,
                font_height - 3
            );
            xlib::XFlush(self.display);
            current_x_pos += (input_width + 8) as i32;

            if status.settings.lines > 0 {
                // Draw vertical matches
                // TODO
            } else {
                // Draw prev icon
                if status.page > 0 {
                    self.draw_text(current_x_pos, font_height as i32, 5, &"<".to_string(), false);
                    current_x_pos += self.text_width(&"<".to_string()) as i32 + 4;
                }

                // Draw horizontal matches
                let (match_items, pages) = self.get_items_page(&status, status.page);
                if pages > status.page + 1 {
                    // Draw next icon and break
                    self.draw_text(self.w as i32 - self.text_width(&">".to_string()) as i32 - 5, font_height as i32, 5, &">".to_string(), false);
                }

                for match_item in match_items {
                    self.draw_text(current_x_pos, font_height as i32, 5, &match_item, *match_item == status.selected);
                    current_x_pos += (self.text_width(&match_item) + 10) as i32;
                }
            }
        }
    }

    fn keypress(&mut self, event: &mut xlib::XKeyPressedEvent, status: &mut super::Status) {
        let mut buf = [0 as i8; 32];
        let mut buf_u8 = [0 as u8; 32];
        let mut ksym: u32;
        let old_text = status.text.clone();

        unsafe {
            xlib::XLookupString(event, buf.as_mut_ptr(), buf.len() as i32, null_mut(), null_mut());
            ksym = xlib::XLookupKeysym(event, 0) as u32;
        }

        for x in 0..32 {
            buf_u8[x] = buf[x] as u8;
        }

        let input = String::from_utf8_lossy(&buf_u8);

        if event.state & xlib::ControlMask != 0 {
            match ksym {
                keysym::XK_a => ksym = keysym::XK_Home,
                keysym::XK_b => ksym = keysym::XK_Left,
                keysym::XK_c => ksym = keysym::XK_Escape,
                keysym::XK_d => ksym = keysym::XK_Delete,
                keysym::XK_e => ksym = keysym::XK_End,
                keysym::XK_f => ksym = keysym::XK_Right,
                keysym::XK_h => ksym = keysym::XK_BackSpace,
                keysym::XK_i => ksym = keysym::XK_Tab,
                keysym::XK_j => ksym = keysym::XK_Return,
                keysym::XK_m => ksym = keysym::XK_Return,
                keysym::XK_n => ksym = keysym::XK_Down,
                keysym::XK_p => ksym = keysym::XK_Up,
                keysym::XK_k => {
                    status.text.remove(self.cursor);
                },
                keysym::XK_u => {
                    status.text.remove(self.cursor);
                },
                keysym::XK_w => {
                    while self.cursor > 0 && status.text.remove(self.cursor) == ' ' { self.cursor -= 1; }
                    while self.cursor > 0 && status.text.remove(self.cursor) != ' ' { self.cursor -= 1; }
                },
                // TODO: Understand and implement it
                // keysym::XK_y => {
                //     unsafe {
                //         xlib::XConvertSelection(self.display, if (event.state & xlib::ShiftMask != 0) { clip } else { xlib::XA_PRIMARY }, utf8, utf8, win, xlib::CurrentTime);
                //     }
                //     return;
                // },
                _ => return,
            }
        } else if event.state & xlib::Mod1Mask != 0 {
            match ksym  {
                keysym::XK_g => ksym = keysym::XK_Home,
                keysym::XK_G => ksym = keysym::XK_End,
                keysym::XK_h => ksym = keysym::XK_Up,
                keysym::XK_j => ksym = keysym::XK_Next,
                keysym::XK_k => ksym = keysym::XK_Prior,
                keysym::XK_l => ksym = keysym::XK_Down,
                _ => return,
            }
        }

        match ksym {
            keysym::XK_Delete => {
                if status.text.len() >= self.cursor {
                    status.text.remove(self.cursor);
                }
            },
            keysym::XK_BackSpace => {
                if self.cursor > 0 && status.text.len() > 0 {
                    status.text.remove(self.cursor-1);
                    self.cursor -= 1;
                }
            },
            keysym::XK_End => {
                if self.cursor < status.text.len() {
                    self.cursor = status.text.len();
                } else {
                    status.selected = status.matches.last().unwrap_or(&"".to_string()).clone();
                }
            },
            keysym::XK_Escape => { unsafe { exit(0 as c_int); } },
            keysym::XK_Home => {
                if status.selected == status.matches.first().unwrap_or(&"".to_string()).clone() {
                    self.cursor = 0;
                } else {
                    status.selected = status.matches.first().unwrap_or(&"".to_string()).clone();
                }
            },
            keysym::XK_Left => {
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
            keysym::XK_Up => {
                match status.matches.binary_search(&status.selected) {
                    Ok(0) => return,
                    Ok(n) => status.selected = status.matches[n - 1].clone(),
                    Err(_) => return
                }
            },
            keysym::XK_Next => {
                // TODO: Calc the number of pages
                status.page += 1
            },
            keysym::XK_Prior => {
                if status.page > 0 {
                    status.page -= 1
                }
            },
            keysym::XK_Return => {
                if (event.state & xlib::ShiftMask) != 0 || status.selected == "" {
                    println!("{}", status.text)
                } else {
                    println!("{}", status.selected)
                }
                unsafe { exit(0 as c_int) }
            },
            keysym::XK_KP_Enter => {
                if (event.state & xlib::ShiftMask) != 0 || status.selected == "" {
                    println!("{}", status.text)
                } else {
                    println!("{}", status.selected)
                }
                unsafe { exit(0 as c_int) }
            },
            keysym::XK_Right => {
                if self.cursor < status.text.len() {
                    self.cursor += 1;
                } else  {
                    match status.matches.binary_search(&status.selected) {
                        Ok(n) => {
                            if n < (status.matches.len() - 1) {
                                status.selected = status.matches[n + 1].clone();
                            } else {
                                return
                            }
                        },
                        Err(_) => return
                    }
                }
            },
            keysym::XK_Down => {
                match status.matches.binary_search(&status.selected) {
                    Ok(n) => {
                        if n < (status.matches.len() - 1) {
                            status.selected = status.matches[n + 1].clone();
                        } else {
                            return
                        }
                    },
                    Err(_) => return
                }
            },
            keysym::XK_Tab => {
                if status.selected != "" {
                    status.text = status.selected.clone();
                    self.cursor = status.text.len();
                }
            },
            _ => unsafe {
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
    }

    // fn paste(&self) {
    //     panic!("Not implemented");
    // }

    pub fn run(&mut self, mut status: super::Status) {
        self.draw_menu(&status);

        unsafe {
            let mut ev: xlib::XEvent = zeroed();

            while !xlib::XNextEvent(self.display, &mut ev) != 0 {
                if xlib::XFilterEvent(&mut ev, self.window) != 0 { continue; }
                match ev.get_type() {
                    xlib::Expose => if xlib::XExposeEvent::from(ev).count == 0 { self.draw_menu(&status); },
                    xlib::KeyPress => self.keypress(&mut xlib::XKeyPressedEvent::from(ev), &mut status),
                    // xlib::SelectionNotify => if xlib::XSelectionEvent::from(ev).property == utf8 { self.paste(); },
                    xlib::VisibilityNotify => if xlib::XVisibilityEvent::from(ev).state != xlib::VisibilityUnobscured { xlib::XRaiseWindow(self.display, self.window); },
                    _ => continue
                }
            }
        }
    }
}

impl Drop for UI {
    fn drop(&mut self) {
        unsafe {
            xlib::XDestroyWindow(self.display, self.window);
            xlib::XCloseDisplay(self.display);
        }
    }
}

