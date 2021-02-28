// TOE Kernel
// Copyright (c) 2021 MEG-OS project

#![no_std]
#![no_main]
#![feature(asm)]

use core::fmt::Write;
use core::time::Duration;
use kernel::arch::cpu::Cpu;
use kernel::fonts::FontManager;
use kernel::graphics::bitmap::*;
use kernel::graphics::color::*;
use kernel::graphics::coords::*;
use kernel::mem::mm::MemoryManager;
use kernel::system::System;
use kernel::task::scheduler::Timer;
use kernel::util::text::*;
use kernel::window::*;
use kernel::*;
use mem::string::*;
use task::scheduler::{Scheduler, SpawnOption};
use window::WindowBuilder;
// use kernel::audio::AudioManager;
// use kernel::util::rng::XorShift32;

entry!(Shell::main);

#[used]
static mut MAIN: Shell = Shell::new();

struct Shell {}

impl Shell {
    const fn new() -> Self {
        Self {}
    }

    fn main() {
        WindowManager::set_desktop_color(IndexedColor::from_rgb(0x2196F3));
        // WindowManager::set_desktop_color(IndexedColor::from_rgb(0x426F96));
        WindowManager::set_pointer_visible(true);
        Timer::sleep(Duration::from_millis(100));

        SpawnOption::new().spawn_f(Self::status_bar_thread, 0, "Status Bar");
        SpawnOption::new().spawn_f(Self::actmon_thread, 0, "Activity Monitor");

        // SpawnOption::new().spawn_f(Self::console_thread, 1, "Command Mode 2");
        Self::console_thread(0);
    }

    fn console_thread(instance: usize) {
        let padding_x = 4;
        let padding_y = 4;
        let font = FontManager::fixed_system_font();
        let bg_color = IndexedColor::WHITE;
        let fg_color = IndexedColor::BLACK;

        let window_rect = Rect::new(
            8 + 136 * instance as isize,
            30,
            128,
            font.line_height() + padding_y * 2,
        );
        let window = WindowBuilder::new("Command Mode")
            .style_add(WindowStyle::NAKED)
            .frame(window_rect)
            .bg_color(bg_color)
            .build();
        window.make_active();

        let interval = 500;
        window.create_timer(0, Duration::from_millis(0));
        let mut sb = Sb255::new();
        let mut cursor_phase = 0;
        while let Some(message) = window.wait_message() {
            match message {
                WindowMessage::Activated | WindowMessage::Deactivated => {
                    window.set_needs_display();
                }
                WindowMessage::Timer(_timer) => {
                    cursor_phase ^= 1;
                    window.create_timer(0, Duration::from_millis(interval));
                    if window.is_active() {
                        window.set_needs_display();
                    }
                }
                WindowMessage::Char(c) => {
                    match c {
                        '\x08' => sb.backspace(),
                        '\x0D' => sb.clear(),
                        _ => {
                            let _ = sb.write_char(c);
                        }
                    }
                    window.set_needs_display();
                }
                WindowMessage::Draw => {
                    window
                        .draw(|bitmap| {
                            let rect = Rect::new(
                                padding_x,
                                padding_y,
                                bitmap.size().width() as isize - padding_x * 2,
                                font.line_height(),
                            );
                            bitmap.view(rect, |bitmap| {
                                bitmap.fill_rect(bitmap.bounds(), bg_color);
                                TextProcessing::write_str(
                                    bitmap,
                                    sb.as_str(),
                                    font,
                                    Point::new(1, 1),
                                    IndexedColor::from_rgb(0xCCCCCC),
                                );
                                TextProcessing::write_str(
                                    bitmap,
                                    sb.as_str(),
                                    font,
                                    Point::new(0, 0),
                                    fg_color,
                                );
                                if window.is_active() && cursor_phase == 1 {
                                    bitmap.fill_rect(
                                        Rect::new(
                                            font.width() * sb.len() as isize,
                                            0,
                                            font.width(),
                                            font.line_height(),
                                        ),
                                        fg_color,
                                    );
                                }
                            });
                        })
                        .unwrap();
                }
                _ => window.handle_default_message(message),
            }
        }
        unimplemented!()
    }

    #[allow(dead_code)]
    fn actmon_thread(_: usize) {
        let window_size = Size::new(280, 160);
        let bg_color = IndexedColor::BLACK;
        let fg_color = IndexedColor::YELLOW;

        let window = WindowBuilder::new("Activity Monitor")
            // .style_add(WindowStyle::FLOATING)
            .frame(Rect::new(
                -window_size.width - 8,
                -window_size.height - 8,
                window_size.width,
                window_size.height,
            ))
            .bg_color(bg_color)
            .build();
        window.show();

        let mut sb = StringBuffer::new();
        let interval = 1000;
        window.create_timer(0, Duration::from_millis(0));
        while let Some(message) = window.wait_message() {
            match message {
                WindowMessage::Timer(_timer) => {
                    window.set_needs_display();
                    window.create_timer(0, Duration::from_millis(interval));
                }
                WindowMessage::Draw => {
                    let font = FontManager::fixed_small_font();
                    sb.clear();
                    writeln!(
                        sb,
                        "Memory {} MB, {} KB Free, {} KB Used",
                        MemoryManager::total_memory_size() >> 20,
                        MemoryManager::free_memory_size() >> 10,
                        (MemoryManager::total_memory_size()
                            - MemoryManager::free_memory_size()
                            - 0x100000)
                            >> 10,
                    )
                    .unwrap();
                    Scheduler::print_statistics(&mut sb, false);

                    window
                        .draw(|bitmap| {
                            bitmap.fill_rect(bitmap.bounds(), window.bg_color());
                            let rect = bitmap.bounds().insets_by(EdgeInsets::new(4, 4, 4, 4));
                            TextProcessing::draw_text(
                                bitmap,
                                sb.as_str(),
                                font,
                                rect,
                                fg_color,
                                0,
                                LineBreakMode::default(),
                                TextAlignment::Left,
                                util::text::VerticalAlignment::Top,
                            );
                        })
                        .unwrap();
                }
                _ => window.handle_default_message(message),
            }
        }
        unimplemented!()
    }

    #[allow(dead_code)]
    fn about_thread(_: usize) {
        let window_size = Size::new(320, 160);
        let window = WindowBuilder::new("About").size(window_size).build();
        window.show();

        let mut sb = StringBuffer::new();
        let interval = 5000;
        window.create_timer(0, Duration::from_millis(0));
        while let Some(message) = window.wait_message() {
            match message {
                WindowMessage::Timer(_timer) => {
                    window.set_needs_display();
                    window.create_timer(0, Duration::from_millis(interval));
                }
                WindowMessage::Draw => {
                    let font = FontManager::fixed_ui_font();
                    sb.clear();

                    writeln!(sb, "{} v{}", System::name(), System::version(),).unwrap();
                    writeln!(sb, "Platform {}", System::platform(),).unwrap();
                    writeln!(sb, "CPU ver {}", System::cpu_ver().0,).unwrap();
                    writeln!(sb, "Memory {} MB", MemoryManager::total_memory_size() >> 20,)
                        .unwrap();

                    window
                        .draw(|bitmap| {
                            bitmap.fill_rect(bitmap.bounds(), window.bg_color());
                            let rect = bitmap.bounds().insets_by(EdgeInsets::new(0, 8, 2, 8));
                            TextProcessing::draw_text(
                                bitmap,
                                sb.as_str(),
                                font,
                                rect,
                                IndexedColor::BLACK,
                                0,
                                LineBreakMode::default(),
                                TextAlignment::Center,
                                util::text::VerticalAlignment::Bottom,
                            );
                        })
                        .unwrap();
                }
                _ => window.handle_default_message(message),
            }
        }
        unimplemented!()
    }

    #[allow(dead_code)]
    fn status_bar_thread(_: usize) {
        const STATUS_BAR_HEIGHT: isize = 24;
        let screen_size = System::main_screen().size();
        let window_size = Size::new(screen_size.width(), STATUS_BAR_HEIGHT);
        let window_rect = Rect::new(0, 0, screen_size.width(), STATUS_BAR_HEIGHT);
        let window = WindowBuilder::new("Status")
            .style(WindowStyle::BORDER | WindowStyle::FLOATING)
            .frame(window_rect)
            // .bg_color(IndexedColor::from_rgb(0xCCCCFF))
            .build();
        window
            .draw_in_rect(window_size.into(), |bitmap| {
                let font = FontManager::fixed_ui_font();
                let s = System::short_name();
                TextProcessing::write_str(
                    bitmap,
                    s,
                    font,
                    Point::new(9, (STATUS_BAR_HEIGHT - font.line_height()) / 2),
                    IndexedColor::BLACK,
                );
            })
            .unwrap();
        window.show();
        WindowManager::add_screen_insets(EdgeInsets::new(STATUS_BAR_HEIGHT, 0, 0, 0));

        SpawnOption::new().spawn_f(Self::about_thread, 0, "About");

        let mut sb = StringBuffer::new();

        window.create_timer(0, Duration::from_millis(0));
        while let Some(message) = window.wait_message() {
            match message {
                WindowMessage::Timer(_timer) => {
                    let time = System::system_time();
                    let interval = 1_000_000_000 - time.nanos as u64;
                    window.create_timer(0, Duration::from_nanos(interval));
                    window.set_needs_display();
                }
                WindowMessage::Draw => {
                    sb.clear();
                    let time = System::system_time();
                    let tod = time.secs % 86400;
                    let min = tod / 60 % 60;
                    let hour = tod / 3600;
                    if true {
                        let sec = tod % 60;
                        if sec % 2 == 0 {
                            write!(sb, "{:2} {:02} {:02}", hour, min, sec).unwrap();
                        } else {
                            write!(sb, "{:2}:{:02}:{:02}", hour, min, sec).unwrap();
                        };
                    } else {
                        write!(sb, "{:2}:{:02}", hour, min).unwrap();
                    }

                    let font = FontManager::fixed_system_font();
                    let clock_width = font.width() * 8;
                    let clock_rect = Rect::new(
                        window_size.width() - clock_width - 8,
                        (window_size.height() - font.line_height()) / 2,
                        clock_width,
                        font.line_height(),
                    );
                    window
                        .draw_in_rect(clock_rect, |bitmap| {
                            bitmap.fill_rect(bitmap.bounds(), window.bg_color());
                            TextProcessing::write_str(
                                bitmap,
                                sb.as_str(),
                                font,
                                Point::default(),
                                IndexedColor::BLACK,
                            );
                        })
                        .unwrap();
                    window.invalidate_rect(clock_rect);
                }
                _ => window.handle_default_message(message),
            }
        }
        unimplemented!()
    }
}

// const BITMAP_WIDTH: isize = 16;
// const BITMAP_HEIGHT: isize = 16;
// static BITMAP_DATA: [u8; (BITMAP_WIDTH * BITMAP_HEIGHT) as usize] = [
//     0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
//     0xFF, 0xFF, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0xFF, 0xFF,
//     0xFF, 0xFF, 0x02, 0x0A, 0x0A, 0x0A, 0x0A, 0x0A, 0x0A, 0x0A, 0x0A, 0x0A, 0x0A, 0x02, 0xFF, 0xFF,
//     0xFF, 0xFF, 0x02, 0x0A, 0x00, 0x00, 0x0A, 0x0A, 0x0A, 0x0A, 0x00, 0x00, 0x0A, 0x02, 0xFF, 0xFF,
//     0xFF, 0xFF, 0x02, 0x0A, 0x00, 0x00, 0x0A, 0x0A, 0x0A, 0x0A, 0x00, 0x00, 0x0A, 0x02, 0xFF, 0xFF,
//     0xFF, 0xFF, 0x02, 0x0A, 0x0A, 0x0A, 0x0A, 0x00, 0x00, 0x0A, 0x0A, 0x0A, 0x0A, 0x02, 0xFF, 0xFF,
//     0xFF, 0xFF, 0x02, 0x0A, 0x0A, 0x0A, 0x00, 0x00, 0x00, 0x00, 0x0A, 0x0A, 0x0A, 0x02, 0xFF, 0xFF,
//     0xFF, 0xFF, 0x02, 0x0A, 0x0A, 0x0A, 0x00, 0x0A, 0x0A, 0x00, 0x0A, 0x0A, 0x0A, 0x02, 0xFF, 0xFF,
//     0xFF, 0xFF, 0x02, 0x0A, 0x0A, 0x0A, 0x0A, 0x0A, 0x0A, 0x0A, 0x0A, 0x0A, 0x0A, 0x02, 0xFF, 0xFF,
//     0xFF, 0xFF, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0xFF, 0xFF,
//     0xFF, 0xFF, 0xFF, 0x0F, 0x00, 0x0F, 0x00, 0x0F, 0x00, 0x0F, 0x0F, 0x00, 0x0F, 0xFF, 0xFF, 0xFF,
//     0xFF, 0xFF, 0xFF, 0x0F, 0x00, 0x0F, 0x00, 0x0F, 0x00, 0x0F, 0x0F, 0x00, 0x0F, 0xFF, 0xFF, 0xFF,
//     0xFF, 0x0F, 0x0F, 0x00, 0x0F, 0x0F, 0x00, 0x0F, 0x00, 0x0F, 0x0F, 0x00, 0x0F, 0xFF, 0x0F, 0xFF,
//     0x0F, 0x00, 0x0F, 0x00, 0x0F, 0x00, 0x0F, 0x0F, 0x00, 0x0F, 0x0F, 0x00, 0x0F, 0x0F, 0x00, 0x0F,
//     0xFF, 0x0F, 0x00, 0x0F, 0x0F, 0x00, 0x0F, 0x0F, 0x00, 0x0F, 0xFF, 0x0F, 0x00, 0x00, 0x0F, 0xFF,
//     0xFF, 0xFF, 0x0F, 0xFF, 0xFF, 0x0F, 0xFF, 0xFF, 0x0F, 0xFF, 0xFF, 0xFF, 0x0F, 0x0F, 0xFF, 0xFF,
// ];
