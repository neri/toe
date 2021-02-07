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
use kernel::window::*;
use kernel::*;
use mem::string::*;
use task::scheduler::{Scheduler, SpawnOption};
use window::WindowBuilder;
// use kernel::audio::AudioManager;
// use kernel::util::rng::XorShift32;

entry!(Application::main);

#[used]
static mut MAIN: Application = Application::new();

struct Application {}

impl Application {
    const fn new() -> Self {
        Self {}
    }

    fn main() {
        WindowManager::set_desktop_color(IndexedColor::from_rgb(0x2196F3));
        WindowManager::set_pointer_visible(true);

        {
            let screen_size = System::main_screen().size();
            let window_size = Size::new(screen_size.width(), 24);
            let window = WindowBuilder::new("Status")
                .style(WindowStyle::BORDER | WindowStyle::FLOATING)
                .frame(window_size.into())
                .build();
            window
                .draw_in_rect(window_size.into(), |bitmap| {
                    let font = FontManager::fixed_system_font();
                    font.write_str(
                        System::name(),
                        bitmap,
                        Point::new(4, 2),
                        IndexedColor::BLACK,
                    );
                })
                .unwrap();
            window.show();
        }

        {
            let window_size = Size::new(240, 150);
            let window = WindowBuilder::new("Hello").size(window_size).build();
            window
                .draw_in_rect(window_size.into(), |bitmap| {
                    let font = FontManager::fixed_system_font();
                    font.write_str("It works!", bitmap, Point::new(10, 10), IndexedColor::BLACK);
                    // let rect = Rect::new(40, 60, 160, 20);
                    // let radius = 8;
                    // bitmap.fill_round_rect(rect, radius, IndexedColor::LIGHT_BLUE);
                    // bitmap.draw_round_rect(rect, radius, IndexedColor::BLACK);
                })
                .unwrap();
            window.make_active();
        }

        println!("\n\n");
        println!("{} v{}", System::name(), System::version(),);
        println!(
            "Platform {} CPU Gen {}",
            System::platform(),
            System::cpu_ver().0,
        );
        println!(
            "Memory {} KB Free, {} MB Total",
            MemoryManager::free_memory_size() >> 10,
            MemoryManager::total_memory_size() >> 20
        );

        for i in 0..5 {
            SpawnOption::new().spawn(Self::thread_test, i, "test");
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

    fn thread_test(i: usize) {
        let window = WindowBuilder::new("Thread Test")
            .frame(Rect::new(-160, 40 + i as isize * 60, 150, 50))
            .build();
        window.show();

        let mut sb = StringBuffer::new();
        let mut counter = 0;
        loop {
            sformat!(sb, "{}", counter);

            window
                .draw(|bitmap| {
                    bitmap.fill_rect(bitmap.bounds(), IndexedColor::WHITE);
                    let font = FontManager::fixed_system_font();
                    font.write_str(sb.as_str(), bitmap, Point::new(10, 0), IndexedColor::BLACK);
                })
                .unwrap();

            Timer::sleep(Duration::from_millis(100));

            counter += 1;
        }
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
