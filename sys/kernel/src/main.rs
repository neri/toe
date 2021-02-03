// TOE Kernel
// Copyright (c) 2021 MEG-OS project

#![no_std]
#![no_main]
#![feature(asm)]

use core::{alloc::Layout, fmt::Write, time::Duration};
use kernel::arch::cpu::Cpu;
use kernel::audio::AudioManager;
use kernel::fonts::FontManager;
use kernel::graphics::bitmap::*;
use kernel::graphics::color::*;
use kernel::graphics::coords::*;
use kernel::mem::mm::MemoryManager;
use kernel::system::System;
use kernel::task::scheduler::Timer;
use kernel::util::rng::XorShift32;
use kernel::window::*;
use kernel::*;
use window::WindowBuilder;

entry!(Application::main);

#[used]
static mut MAIN: Application = Application::new();

struct Application {}

impl Application {
    const fn new() -> Self {
        Self {}
    }

    fn main() {
        let bitmap = System::main_screen();
        let size = bitmap.size();

        bitmap.fill_rect(Rect::from(size), IndexedColor::from_rgb(0x802196F3));

        {
            let screen_size = System::main_screen().size();
            let window_size = Size::new(screen_size.width(), 24);
            let mut window = WindowBuilder::new("Status")
                .style(WindowStyle::BORDER | WindowStyle::FLOATING)
                .frame(window_size.into())
                .build_inner();
            window
                .draw_in_rect(window_size.into(), |bitmap| {
                    let font = FontManager::fixed_system_font();
                    font.write_str("12:34:56", bitmap, Point::new(4, 2), IndexedColor::BLACK);
                })
                .unwrap();
            window.draw_frame();
            window.draw_to_screen(window.bounds());
        }

        {
            let window_size = Size::new(240, 96);
            let mut window = WindowBuilder::new("Hello").size(window_size).build_inner();
            window.draw_frame();
            window
                .draw_in_rect(window_size.into(), |bitmap| {
                    let font = FontManager::fixed_system_font();
                    font.write_str("It works!", bitmap, Point::new(10, 10), IndexedColor::BLACK);
                    let rect = Rect::new(40, 40, 160, 15);
                    let radius = 8;
                    bitmap.fill_round_rect(rect, radius, IndexedColor::LIGHT_BLUE);
                    bitmap.draw_round_rect(rect, radius, IndexedColor::BLACK);
                })
                .unwrap();
            window.draw_to_screen(window.bounds());
        }

        // println!("\n\n");
        println!("{} v{}", System::name(), System::version(),);
        println!("Platform {}", System::platform(),);
        println!(
            "Memory {} KB Free, {} MB Total",
            MemoryManager::free_memory_size() >> 10,
            MemoryManager::total_memory_size() >> 20
        );

        if false {
            let screen = bitmap;
            let mut rng = XorShift32::default();
            let bitmap =
                OsBitmap8::from_bytes(&BITMAP_DATA, Size::new(BITMAP_WIDTH, BITMAP_HEIGHT));
            for _ in 0..100 {
                let x = (rng.next() % 300) as isize;
                let y = (rng.next() % 180) as isize;
                screen.blt_with_key(
                    &bitmap,
                    Point::new(x, y),
                    bitmap.bounds(),
                    IndexedColor(0xFF),
                );
            }
        }

        print!("# ");
        loop {
            if let Some(key) = WindowManager::get_key() {
                match key {
                    '\x08' => print!(" \x08\x08"),
                    '\r' => print!(" \n# "),
                    _ => print!("{}", key),
                }
            } else {
                print!("\x7F\x08");
                unsafe {
                    Cpu::halt();
                }
            }
        }
    }
}

const BITMAP_WIDTH: isize = 16;
const BITMAP_HEIGHT: isize = 16;
static BITMAP_DATA: [u8; (BITMAP_WIDTH * BITMAP_HEIGHT) as usize] = [
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0xFF, 0xFF,
    0xFF, 0xFF, 0x02, 0x0A, 0x0A, 0x0A, 0x0A, 0x0A, 0x0A, 0x0A, 0x0A, 0x0A, 0x0A, 0x02, 0xFF, 0xFF,
    0xFF, 0xFF, 0x02, 0x0A, 0x00, 0x00, 0x0A, 0x0A, 0x0A, 0x0A, 0x00, 0x00, 0x0A, 0x02, 0xFF, 0xFF,
    0xFF, 0xFF, 0x02, 0x0A, 0x00, 0x00, 0x0A, 0x0A, 0x0A, 0x0A, 0x00, 0x00, 0x0A, 0x02, 0xFF, 0xFF,
    0xFF, 0xFF, 0x02, 0x0A, 0x0A, 0x0A, 0x0A, 0x00, 0x00, 0x0A, 0x0A, 0x0A, 0x0A, 0x02, 0xFF, 0xFF,
    0xFF, 0xFF, 0x02, 0x0A, 0x0A, 0x0A, 0x00, 0x00, 0x00, 0x00, 0x0A, 0x0A, 0x0A, 0x02, 0xFF, 0xFF,
    0xFF, 0xFF, 0x02, 0x0A, 0x0A, 0x0A, 0x00, 0x0A, 0x0A, 0x00, 0x0A, 0x0A, 0x0A, 0x02, 0xFF, 0xFF,
    0xFF, 0xFF, 0x02, 0x0A, 0x0A, 0x0A, 0x0A, 0x0A, 0x0A, 0x0A, 0x0A, 0x0A, 0x0A, 0x02, 0xFF, 0xFF,
    0xFF, 0xFF, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0x0F, 0x00, 0x0F, 0x00, 0x0F, 0x00, 0x0F, 0x0F, 0x00, 0x0F, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0x0F, 0x00, 0x0F, 0x00, 0x0F, 0x00, 0x0F, 0x0F, 0x00, 0x0F, 0xFF, 0xFF, 0xFF,
    0xFF, 0x0F, 0x0F, 0x00, 0x0F, 0x0F, 0x00, 0x0F, 0x00, 0x0F, 0x0F, 0x00, 0x0F, 0xFF, 0x0F, 0xFF,
    0x0F, 0x00, 0x0F, 0x00, 0x0F, 0x00, 0x0F, 0x0F, 0x00, 0x0F, 0x0F, 0x00, 0x0F, 0x0F, 0x00, 0x0F,
    0xFF, 0x0F, 0x00, 0x0F, 0x0F, 0x00, 0x0F, 0x0F, 0x00, 0x0F, 0xFF, 0x0F, 0x00, 0x00, 0x0F, 0xFF,
    0xFF, 0xFF, 0x0F, 0xFF, 0xFF, 0x0F, 0xFF, 0xFF, 0x0F, 0xFF, 0xFF, 0xFF, 0x0F, 0x0F, 0xFF, 0xFF,
];
