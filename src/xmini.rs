use std::ffi::CString;
use std::str::from_utf8;
use std::thread::sleep;
use std::env;
use std::time::Duration;
use std::ptr::{null_mut, read};
use std::mem::zeroed;
use std::convert::From;

use libc::c_uint;

use x11::xlib;

pub struct ColorMap {
    pointer: u64,
}

pub struct Screen {
    pointer: i32,
    display_pointer: *mut xlib::Display,
}

impl Screen {
    pub fn get_root_window(&self) -> Window {
        unsafe {
            Window {
                pointer: xlib::XRootWindow(self.display_pointer, self.pointer),
                display_pointer: self.display_pointer
            }
        }
    }

    pub fn get_default_colormap(&self) -> ColorMap {
        unsafe {
            ColorMap {pointer: xlib::XDefaultColormap(self.display_pointer, self.pointer)}
        }
    }

    pub fn get_geometry(&self, xfont: &XFontStruct) -> (u32, u32) {
        unsafe {
            let width = xlib::XDisplayWidth(self.display_pointer, self.pointer) as u32;
            let height = (read(xfont.pointer).max_bounds.ascent + read(xfont.pointer).max_bounds.descent + 4) as u32;
            (width, height)
        }
    }
}

pub struct Color {
    pointer: xlib::XColor
}

impl Clone for Color {
    fn clone(&self) -> Self {
        Color { pointer: self.pointer.clone() }
    }
}

pub struct Display {
    pointer: *mut xlib::Display,
    copy: bool
}

impl Drop for Display {
    fn drop(&mut self) {
        if !self.copy {
            unsafe {
                xlib::XCloseDisplay(self.pointer);
            }
        }
    }
}

impl Display {
    pub fn new() -> Display {
        let display_env;
        match env::var("DISPLAY") {
            Ok(val) => display_env = CString::new(val).unwrap(),
            Err(_) => display_env = CString::new("").unwrap(),
        }

        let display;
        unsafe {
            display = xlib::XOpenDisplay(display_env.as_ptr());
        }

        if display == null_mut() {
            panic!("Cannot connect to X Server: {}", from_utf8(display_env.as_bytes()).unwrap());
        }
        Display { pointer: display , copy: false }
    }

    fn new_from_ptr(ptr: *mut xlib::Display) -> Display {
        Display { pointer: ptr, copy: true }
    }

    pub fn default_root_window(&self) -> Window {
        unsafe {
            Window {
                pointer: xlib::XDefaultRootWindow(self.pointer),
                display_pointer: self.pointer
            }
        }
    }

    pub fn grab_keyboard(&self) -> bool {
        unsafe {
            xlib::XGrabKeyboard(self.pointer, self.default_root_window().pointer, 1, xlib::GrabModeAsync, xlib::GrabModeAsync, xlib::CurrentTime) == xlib::GrabSuccess
        }
    }

    pub fn get_default_screen(&self) -> Screen {
        unsafe {
            Screen {
                pointer: xlib::XDefaultScreen(self.pointer),
                display_pointer: self.pointer
            }
        }
    }


    pub fn alloc_named_color(&self, color_map: &ColorMap, color_name: &String) -> Color {
        unsafe {
            let color_name = CString::new(color_name.clone()).unwrap();
            let mut color: xlib::XColor = zeroed();
            xlib::XAllocNamedColor(self.pointer, color_map.pointer, color_name.as_ptr(), &mut color, &mut color);
            Color { pointer: color }
        }
    }

    pub fn flush(&self) {
        unsafe {
            xlib::XFlush(self.pointer);
        }
    }

    pub fn new_window(&self, parent_window: &Window, width: u32, height: u32, color_bg: &Color) -> Window {
        unsafe {
            let mut attributes: xlib::XSetWindowAttributes = zeroed();
            attributes.background_pixel = color_bg.pointer.pixel;
            attributes.override_redirect = 1;
            attributes.event_mask =  xlib::StructureNotifyMask | xlib::ExposureMask | xlib::KeyPressMask | xlib::VisibilityChangeMask;


            let window = Window {
                pointer: xlib::XCreateWindow(self.pointer, parent_window.pointer, 0, 0, width, height, 0, 0,
                                             xlib::InputOutput as c_uint, null_mut(),
                                             xlib::CWOverrideRedirect | xlib::CWBackPixel | xlib::CWEventMask, &mut attributes),
                display_pointer: self.pointer
            };

            // Show window
            window.map();
            self.flush();
            window
        }
    }

    pub fn new_font(&self, font_name: &String) -> XFontStruct {
        unsafe{
            let fontstr = CString::new(font_name.clone()).unwrap();
            XFontStruct {
                pointer: xlib::XLoadQueryFont(self.pointer, fontstr.as_ptr())
            }
        }
    }

    pub fn sync(&self, discard: bool) {
        unsafe {
            if discard {
                xlib::XSync(self.pointer, 1);
            } else {
                xlib::XSync(self.pointer, 0);
            }
        }
    }

    pub fn wait_keyboard(&self) {
        /* try to grab keyboard, we may have to wait for another process to ungrab */
        for _ in 1..1000 {
            if self.grab_keyboard() {
                return
            }
            sleep(Duration::new(1, 0));
        }
        panic!("cannot grab keyboard");
    }

    pub fn wait_until_map_notify(&self) {
        loop {
            match self.next_event() {
                Some(event) => if event.get_type() == xlib::MapNotify { break },
                None => ()
            }
        }
    }

    pub fn next_event(&self) -> Option<Event> {
        unsafe {
            let mut event: xlib::XEvent = zeroed();
            let result = xlib::XNextEvent(self.pointer, &mut event);
            if result == 0 { Some(Event { pointer: event}) } else { None }
        }
    }
}

pub struct Window {
    pointer: xlib::Window,
    display_pointer: *mut xlib::Display,
}

impl Drop for Window {
    fn drop(&mut self) {
        unsafe {
            xlib::XDestroyWindow(self.display_pointer, self.pointer);
        }
    }
}

impl Window {
    pub fn map(&self) {
        unsafe {
            xlib::XMapWindow(self.display_pointer, self.pointer);
        }
    }

    pub fn new_child_window(&self, width: u32, height: u32, color_bg: &Color) -> Window {
        Display::new_from_ptr(self.display_pointer).new_window(self, width, height, color_bg)
    }

    pub fn new_gc(&self, color_fg: Color, color_bg: Color) -> GC {
        unsafe {
            let mut values: xlib::XGCValues = zeroed();
            let valuesmask: u64 = 0 as u64;

            let gc = GC {
                pointer: xlib::XCreateGC(self.display_pointer, self.pointer, valuesmask, &mut values),
                display_pointer: self.display_pointer,
                window_pointer: self.pointer
            };

            gc.set_foreground(&color_fg);
            gc.set_background(&color_bg);

            gc.set_line_attributes(1, xlib::LineSolid, xlib::CapButt, xlib::JoinMiter);
            gc.set_fill_style(xlib::FillSolid);

            Display::new_from_ptr(self.display_pointer).sync(true);
            Display::new_from_ptr(self.display_pointer).flush();
            gc
        }
    }

    pub fn raise(&self) {
        unsafe {
            xlib::XRaiseWindow(self.display_pointer, self.pointer);
        }
    }
}

pub struct GC {
    pointer: xlib::GC,
    display_pointer: *mut xlib::Display,
    window_pointer: xlib::Window
}

impl GC {
    pub fn set_background(&self, color: &Color) {
        unsafe {
            xlib::XSetBackground(self.display_pointer, self.pointer, color.pointer.pixel);
        }
        ()
    }
    pub fn set_foreground(&self, color: &Color) {
        unsafe {
            xlib::XSetForeground(self.display_pointer, self.pointer, color.pointer.pixel);
        }
        ()
    }
    pub fn fill_rectangle(&self, x: i32, y: i32, w: u32, h: u32) {
        unsafe {
            xlib::XFillRectangle(self.display_pointer, self.window_pointer, self.pointer, x, y, w, h);
        }
    }
    pub fn draw_rectangle(&self, x: i32, y: i32, w: u32, h: u32) {
        unsafe {
            xlib::XDrawRectangle(self.display_pointer, self.window_pointer, self.pointer, x, y, w, h);
        }
    }
    pub fn set_fill_style(&self, style: i32) {
        unsafe {
            xlib::XSetFillStyle(self.display_pointer, self.pointer, style);
        }
    }
    pub fn set_line_attributes(&self, line_width: u32, line_style: i32, cap_style: i32, join_style: i32) {
        unsafe {
            xlib::XSetLineAttributes(self.display_pointer, self.pointer, line_width, line_style, cap_style, join_style);
        }
    }

    pub fn set_font(&self, font: &XFontStruct) {
        unsafe {
            xlib::XSetFont(self.display_pointer, self.pointer, read(font.pointer).fid);
        }
    }

    pub fn draw_string(&self, x: i32, y: i32, text: &String) {
        unsafe {
            xlib::XDrawString(self.display_pointer, self.window_pointer, self.pointer, x + 5, y, CString::new(text.clone()).unwrap().as_ptr(), text.len() as i32);
        }
    }
}

pub struct XFontStruct {
    pointer: *mut xlib::XFontStruct,
}

impl XFontStruct {
    pub fn text_width(&self, text: &String) -> u32 {
        text.len() as u32 * self.font_width()
    }

    pub fn text_height(&self) -> u32 {
        self.font_height()
    }

    pub fn font_width(&self) -> u32 {
        unsafe{
            (read(self.pointer).max_bounds.rbearing - read(self.pointer).min_bounds.lbearing) as u32
        }
    }

    pub fn font_height(&self) -> u32 {
        unsafe{
            (read(self.pointer).max_bounds.ascent + read(self.pointer).max_bounds.descent) as u32
        }
    }
}

pub struct Event {
    pointer: xlib::XEvent,
}

pub struct ExposeEvent { pointer: xlib::XExposeEvent }

impl ExposeEvent {
    pub fn count(&self) -> i32 { self.pointer.count }
}

pub struct KeyPressedEvent { pointer: xlib::XKeyPressedEvent }

impl KeyPressedEvent {
    pub fn state(&self) -> u32 { self.pointer.state }

    pub fn lookup_keysym(&mut self) -> u32 {
        unsafe { xlib::XLookupKeysym(&mut self.pointer, 0) as u32 }
    }

    pub fn lookup_string(&mut self) -> String {
        let mut buf = [0 as i8; 32];
        let mut buf_u8 = [0 as u8; 32];

        unsafe {
            xlib::XLookupString(&mut self.pointer, buf.as_mut_ptr(), buf.len() as i32, null_mut(), null_mut());
        }

        for x in 0..32 {
            buf_u8[x] = buf[x] as u8;
        }

        String::from_utf8_lossy(&buf_u8).into_owned()
    }
}

pub struct SelectionEvent { pointer: xlib::XSelectionEvent }

impl SelectionEvent {
    pub fn property(&self) -> u64 { self.pointer.property }
}

pub struct VisibilityEvent { pointer: xlib::XVisibilityEvent }

impl VisibilityEvent {
    pub fn state(&self) -> i32 { self.pointer.state }
}

impl Event {
    pub fn filter_event(&mut self, window: &Window) -> bool {
        unsafe {
            xlib::XFilterEvent(&mut self.pointer, window.pointer) != 0
        }
    }

    pub fn get_type(&self) -> i32 {
        xlib::XEvent::get_type(&self.pointer)
    }

    pub fn to_expose_event(&self) -> ExposeEvent {
        ExposeEvent { pointer: xlib::XExposeEvent::from(self.pointer) }
    }

    pub fn to_keypress_event(&self) -> KeyPressedEvent {
        KeyPressedEvent { pointer: xlib::XKeyPressedEvent::from(self.pointer) }
    }

    pub fn to_selection_event(&self) -> SelectionEvent {
        SelectionEvent { pointer: xlib::XSelectionEvent::from(self.pointer) }
    }

    pub fn to_visibility_event(&self) -> VisibilityEvent {
        VisibilityEvent { pointer: xlib::XVisibilityEvent::from(self.pointer) }
    }
}
