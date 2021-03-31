// SVC Function Numbers (AUTO GENERATED)
use core::convert::TryFrom;

#[repr(u32)]
#[derive(Debug, Copy, Clone)]
pub enum Function {
    /// [0]
    Exit = 0,
    /// [1] Display a string
    PrintString = 1,
    /// [2] Get the value of the monotonic timer in microseconds
    Monotonic = 2,
    /// [3] Perform the time service
    Time = 3,
    /// [4] Blocks a thread for the specified microseconds
    Usleep = 4,
    /// [5] Get the system information
    GetSystemInfo = 5,
    /// [6] Create a new window
    NewWindow = 6,
    /// [7] Close a window
    CloseWindow = 7,
    /// [8] Draw a string in a window
    DrawString = 8,
    /// [9] Fill a rectangle in a window
    FillRect = 9,
    /// [10] Draw a rectangle in a window
    DrawRect = 10,
    /// [11] Draw a bitmap in a window
    Blt8 = 11,
    /// [12] Draw a bitmap in a window
    Blt1 = 12,
    /// [13] Draw a bitmap in a window
    Blt32 = 13,
    /// [14] Blend (test)
    BlendRect = 14,
    /// [15] Reflect the window's bitmap to the screen now
    FlashWindow = 15,
    /// [16] Wait for char event
    WaitChar = 16,
    /// [17] Read a char event
    ReadChar = 17,
    /// [100] Return a random number
    Rand = 100,
    /// [101] Set the seed of the random number
    Srand = 101,
    /// [10000] RESERVED
    Alloc = 10000,
    /// [10001] RESERVED
    Free = 10001,
}

impl TryFrom<u32> for Function {
    type Error = ();

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Exit),
            1 => Ok(Self::PrintString),
            2 => Ok(Self::Monotonic),
            3 => Ok(Self::Time),
            4 => Ok(Self::Usleep),
            5 => Ok(Self::GetSystemInfo),
            6 => Ok(Self::NewWindow),
            7 => Ok(Self::CloseWindow),
            8 => Ok(Self::DrawString),
            9 => Ok(Self::FillRect),
            10 => Ok(Self::DrawRect),
            11 => Ok(Self::Blt8),
            12 => Ok(Self::Blt1),
            13 => Ok(Self::Blt32),
            14 => Ok(Self::BlendRect),
            15 => Ok(Self::FlashWindow),
            16 => Ok(Self::WaitChar),
            17 => Ok(Self::ReadChar),
            100 => Ok(Self::Rand),
            101 => Ok(Self::Srand),
            10000 => Ok(Self::Alloc),
            10001 => Ok(Self::Free),
            _ => Err(()),
        }
    }
}
