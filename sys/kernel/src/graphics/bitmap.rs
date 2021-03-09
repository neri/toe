// Drawing library for MEG-OS

use super::color::*;
use super::coords::*;
use alloc::vec::Vec;
use bitflags::*;
use core::cell::UnsafeCell;
use core::convert::TryFrom;
// use core::mem::swap;
use core::mem::transmute;

pub trait Drawable
where
    Self::ColorType: ColorTrait,
{
    type ColorType;

    fn width(&self) -> usize;

    fn height(&self) -> usize;

    fn size(&self) -> Size {
        Size::new(self.width() as isize, self.height() as isize)
    }

    fn bounds(&self) -> Rect {
        Rect::from(self.size())
    }
}

pub trait GetPixel: Drawable {
    unsafe fn get_pixel_unchecked(&self, point: Point) -> Self::ColorType;

    fn get_pixel(&self, point: Point) -> Option<Self::ColorType> {
        if point.is_within(Rect::from(self.size())) {
            Some(unsafe { self.get_pixel_unchecked(point) })
        } else {
            None
        }
    }
}

pub trait SetPixel: Drawable {
    unsafe fn set_pixel_unchecked(&mut self, point: Point, pixel: Self::ColorType);

    fn set_pixel(&mut self, point: Point, pixel: Self::ColorType) {
        if point.is_within(Rect::from(self.size())) {
            unsafe {
                self.set_pixel_unchecked(point, pixel);
            }
        }
    }
}

pub trait RasterImage: Drawable {
    fn slice(&self) -> &[Self::ColorType];

    fn stride(&self) -> usize {
        self.width()
    }
}

impl<T: RasterImage> GetPixel for T {
    /// SAFETY: The point must be within the size range.
    unsafe fn get_pixel_unchecked(&self, point: Point) -> Self::ColorType {
        *self
            .slice()
            .get_unchecked(point.x as usize + point.y as usize * self.stride())
    }
}

pub trait MutableRasterImage: RasterImage {
    fn slice_mut(&mut self) -> &mut [Self::ColorType];
}

impl<T: MutableRasterImage> SetPixel for T {
    /// SAFETY: The point must be within the size range.
    unsafe fn set_pixel_unchecked(&mut self, point: Point, pixel: Self::ColorType) {
        let stride = self.stride();
        *self
            .slice_mut()
            .get_unchecked_mut(point.x as usize + point.y as usize * stride) = pixel;
    }
}

pub trait Blt<T: Drawable>: Drawable {
    fn blt(&mut self, src: &T, origin: Point, rect: Rect);
}

pub trait BasicDrawing: SetPixel {
    fn fill_rect(&mut self, rect: Rect, color: Self::ColorType);
    fn draw_hline(&mut self, origin: Point, width: isize, color: Self::ColorType);
    fn draw_vline(&mut self, origin: Point, height: isize, color: Self::ColorType);

    fn draw_rect(&mut self, rect: Rect, color: Self::ColorType) {
        let coords = match Coordinates::from_rect(rect) {
            Ok(v) => v,
            Err(_) => return,
        };
        let width = rect.width();
        let height = rect.height();
        self.draw_hline(coords.left_top(), width, color);
        self.draw_hline(coords.left_bottom() - Point::new(0, 1), width, color);
        if height > 2 {
            self.draw_vline(coords.left_top() + Point::new(0, 1), height - 2, color);
            self.draw_vline(coords.right_top() + Point::new(-1, 1), height - 2, color);
        }
    }

    fn draw_circle(&mut self, origin: Point, radius: isize, color: Self::ColorType) {
        let rect = Rect {
            origin: origin - radius,
            size: Size::new(radius * 2, radius * 2),
        };
        self.draw_round_rect(rect, radius, color);
    }

    fn fill_circle(&mut self, origin: Point, radius: isize, color: Self::ColorType) {
        let rect = Rect {
            origin: origin - radius,
            size: Size::new(radius * 2, radius * 2),
        };
        self.fill_round_rect(rect, radius, color);
    }

    fn fill_round_rect(&mut self, rect: Rect, radius: isize, color: Self::ColorType) {
        let width = rect.size.width;
        let height = rect.size.height;
        let dx = rect.origin.x;
        let dy = rect.origin.y;

        let mut radius = radius;
        if radius * 2 > width {
            radius = width / 2;
        }
        if radius * 2 > height {
            radius = height / 2;
        }

        let lh = height - radius * 2;
        if lh > 0 {
            let rect_line = Rect::new(dx, dy + radius, width, lh);
            self.fill_rect(rect_line, color);
        }

        let mut cx = radius;
        let mut cy = 0;
        let mut f = -2 * radius + 3;
        let qh = height - 1;

        while cx >= cy {
            {
                let bx = radius - cy;
                let by = radius - cx;
                let dw = width - bx * 2;
                self.draw_hline(Point::new(dx + bx, dy + by), dw, color);
                self.draw_hline(Point::new(dx + bx, dy + qh - by), dw, color);
            }

            {
                let bx = radius - cx;
                let by = radius - cy;
                let dw = width - bx * 2;
                self.draw_hline(Point::new(dx + bx, dy + by), dw, color);
                self.draw_hline(Point::new(dx + bx, dy + qh - by), dw, color);
            }

            if f >= 0 {
                cx -= 1;
                f -= 4 * cx;
            }
            cy += 1;
            f += 4 * cy + 2;
        }
    }

    fn draw_round_rect(&mut self, rect: Rect, radius: isize, color: Self::ColorType) {
        let width = rect.size.width;
        let height = rect.size.height;
        let dx = rect.origin.x;
        let dy = rect.origin.y;

        let mut radius = radius;
        if radius * 2 > width {
            radius = width / 2;
        }
        if radius * 2 > height {
            radius = height / 2;
        }

        let lh = height - radius * 2;
        if lh > 0 {
            self.draw_vline(Point::new(dx, dy + radius), lh, color);
            self.draw_vline(Point::new(dx + width - 1, dy + radius), lh, color);
        }
        let lw = width - radius * 2;
        if lw > 0 {
            self.draw_hline(Point::new(dx + radius, dy), lw, color);
            self.draw_hline(Point::new(dx + radius, dy + height - 1), lw, color);
        }

        let mut cx = radius;
        let mut cy = 0;
        let mut f = -2 * radius + 3;
        let qh = height - 1;

        while cx >= cy {
            {
                let bx = radius - cy;
                let by = radius - cx;
                let dw = width - bx * 2 - 1;
                self.set_pixel(Point::new(dx + bx, dy + by), color);
                self.set_pixel(Point::new(dx + bx, dy + qh - by), color);
                self.set_pixel(Point::new(dx + bx + dw, dy + by), color);
                self.set_pixel(Point::new(dx + bx + dw, dy + qh - by), color);
            }

            {
                let bx = radius - cx;
                let by = radius - cy;
                let dw = width - bx * 2 - 1;
                self.set_pixel(Point::new(dx + bx, dy + by), color);
                self.set_pixel(Point::new(dx + bx, dy + qh - by), color);
                self.set_pixel(Point::new(dx + bx + dw, dy + by), color);
                self.set_pixel(Point::new(dx + bx + dw, dy + qh - by), color);
            }

            if f >= 0 {
                cx -= 1;
                f -= 4 * cx;
            }
            cy += 1;
            f += 4 * cy + 2;
        }
    }

    fn draw_line(&mut self, c1: Point, c2: Point, color: Self::ColorType) {
        if c1.x() == c2.x() {
            if c1.y() < c2.y() {
                let height = c2.y() - c1.y();
                self.draw_vline(c1, height, color);
            } else {
                let height = c1.y() - c2.y();
                self.draw_vline(c2, height, color);
            }
        } else if c1.y() == c2.y() {
            if c1.x() < c2.x() {
                let width = c2.x() - c1.x();
                self.draw_hline(c1, width, color);
            } else {
                let width = c1.x() - c2.x();
                self.draw_hline(c2, width, color);
            }
        } else {
            c1.line_to(c2, |point| {
                self.set_pixel(point, color);
            });
        }
    }
}

pub trait RasterFontWriter: SetPixel {
    fn draw_font(&mut self, src: &[u8], size: Size, origin: Point, color: Self::ColorType) {
        let stride = (size.width as usize + 7) / 8;

        let mut coords = match Coordinates::from_rect(Rect { origin, size }) {
            Ok(v) => v,
            Err(_) => return,
        };

        let width = self.width() as isize;
        let height = self.height() as isize;
        if coords.right > width {
            coords.right = width;
        }
        if coords.bottom > height {
            coords.bottom = height;
        }
        if coords.left < 0 || coords.left >= width || coords.top < 0 || coords.top >= height {
            return;
        }

        let new_rect = Rect::from(coords);
        let width = new_rect.width() as usize;
        let height = new_rect.height();
        let w8 = width / 8;
        let w7 = width & 7;
        let mut cursor = 0;
        for y in 0..height {
            for i in 0..w8 {
                let data = unsafe { src.get_unchecked(cursor + i) };
                for j in 0..8 {
                    let position = 0x80u8 >> j;
                    if (data & position) != 0 {
                        let x = (i * 8 + j) as isize;
                        let y = y;
                        let point = Point::new(origin.x + x, origin.y + y);
                        self.set_pixel(point, color);
                    }
                }
            }
            if w7 > 0 {
                let data = unsafe { src.get_unchecked(cursor + w8) };
                let base_x = w8 * 8;
                for i in 0..w7 {
                    let position = 0x80u8 >> i;
                    if (data & position) != 0 {
                        let x = (i + base_x) as isize;
                        let y = y;
                        let point = Point::new(origin.x + x, origin.y + y);
                        self.set_pixel(point, color);
                    }
                }
            }
            cursor += stride;
        }
    }
}

//-//-//-//-//-//-//-//-//-//-//-//-//-//-//-//-//-//-//-//-//-//-//-//-//-//-//-//-//-//-//-//

#[repr(C)]
pub struct ConstBitmap8<'a> {
    width: usize,
    height: usize,
    stride: usize,
    slice: &'a [IndexedColor],
}

impl<'a> ConstBitmap8<'a> {
    #[inline]
    pub const fn from_slice(slice: &'a [IndexedColor], size: Size, stride: usize) -> Self {
        Self {
            width: size.width() as usize,
            height: size.height() as usize,
            stride,
            slice,
        }
    }

    #[inline]
    pub const fn from_bytes(bytes: &'a [u8], size: Size) -> Self {
        Self {
            width: size.width() as usize,
            height: size.height() as usize,
            stride: size.width() as usize,
            slice: unsafe { transmute(bytes) },
        }
    }

    #[inline]
    pub fn clone(&'a self) -> Self {
        Self {
            width: self.width(),
            height: self.height(),
            stride: self.stride(),
            slice: self.slice(),
        }
    }
}

impl Drawable for ConstBitmap8<'_> {
    type ColorType = IndexedColor;

    fn width(&self) -> usize {
        self.width
    }

    fn height(&self) -> usize {
        self.height
    }
}

impl RasterImage for ConstBitmap8<'_> {
    fn stride(&self) -> usize {
        self.stride
    }

    fn slice(&self) -> &[Self::ColorType] {
        self.slice
    }
}

#[repr(C)]
pub struct Bitmap8<'a> {
    width: usize,
    height: usize,
    stride: usize,
    slice: UnsafeCell<&'a mut [IndexedColor]>,
}

impl<'a> Bitmap8<'a> {
    #[inline]
    pub fn from_slice(slice: &'a mut [IndexedColor], size: Size, stride: usize) -> Self {
        Self {
            width: size.width() as usize,
            height: size.height() as usize,
            stride,
            slice: UnsafeCell::new(slice),
        }
    }

    /// Clone a bitmap
    #[inline]
    pub fn clone(&self) -> Bitmap8<'a> {
        let slice = unsafe { self.slice.get().as_mut().unwrap() };
        Self {
            width: self.width(),
            height: self.height(),
            stride: self.stride(),
            slice: UnsafeCell::new(slice),
        }
    }
}

impl Bitmap8<'static> {
    /// SAFETY: Must guarantee the existence of the `ptr`.
    #[inline]
    pub unsafe fn from_static(ptr: *mut IndexedColor, size: Size, stride: usize) -> Self {
        let slice = core::slice::from_raw_parts_mut(ptr, size.height() as usize * stride);
        Self {
            width: size.width() as usize,
            height: size.height() as usize,
            stride,
            slice: UnsafeCell::new(slice),
        }
    }
}

impl BitmapDrawing8 for Bitmap8<'_> {}

pub trait BitmapDrawing8: MutableRasterImage<ColorType = IndexedColor> {
    fn blt<T>(&mut self, src: &T, origin: Point, rect: Rect)
    where
        T: RasterImage<ColorType = <Self as Drawable>::ColorType>,
    {
        self.blt_main(src, origin, rect, None);
    }

    fn blt_with_key<T>(
        &mut self,
        src: &T,
        origin: Point,
        rect: Rect,
        color_key: <Self as Drawable>::ColorType,
    ) where
        T: RasterImage<ColorType = <Self as Drawable>::ColorType>,
    {
        self.blt_main(src, origin, rect, Some(color_key));
    }

    #[inline]
    fn blt_main<T>(
        &mut self,
        src: &T,
        origin: Point,
        rect: Rect,
        color_key: Option<<Self as Drawable>::ColorType>,
    ) where
        T: RasterImage<ColorType = <Self as Drawable>::ColorType>,
    {
        let mut dx = origin.x;
        let mut dy = origin.y;
        let mut sx = rect.origin.x;
        let mut sy = rect.origin.y;
        let mut width = rect.width();
        let mut height = rect.height();

        if dx < 0 {
            sx -= dx;
            width += dx;
            dx = 0;
        }
        if dy < 0 {
            sy -= dy;
            height += dy;
            dy = 0;
        }
        let sw = src.width() as isize;
        let sh = src.height() as isize;
        if width > sx + sw {
            width = sw - sx;
        }
        if height > sy + sh {
            height = sh - sy;
        }
        let r = dx + width;
        let b = dy + height;
        let dw = self.width() as isize;
        let dh = self.height() as isize;
        if r >= dw {
            width = dw - dx;
        }
        if b >= dh {
            height = dh - dy;
        }
        if width <= 0 || height <= 0 {
            return;
        }

        let width = width as usize;
        let height = height as usize;

        let ds = self.stride();
        let ss = src.stride();
        let mut dest_cursor = dx as usize + dy as usize * ds;
        let mut src_cursor = sx as usize + sy as usize * ss;
        let dest_fb = self.slice_mut();
        let src_fb = src.slice();

        if let Some(color_key) = color_key {
            for _ in 0..height {
                for i in 0..width {
                    let c = src_fb[src_cursor + i];
                    if c != color_key {
                        dest_fb[dest_cursor + i] = c;
                    }
                }
                dest_cursor += ds;
                src_cursor += ss;
            }
        } else {
            if ds == width && ss == width {
                memcpy_colors8(dest_fb, dest_cursor, src_fb, src_cursor, width * height);
            } else {
                for _ in 0..height {
                    memcpy_colors8(dest_fb, dest_cursor, src_fb, src_cursor, width);
                    dest_cursor += ds;
                    src_cursor += ss;
                }
            }
        }
    }

    /// Make a bitmap view
    fn view<'a, F, R>(&'a mut self, rect: Rect, f: F) -> Option<R>
    where
        F: FnOnce(&mut Bitmap8) -> R,
    {
        let coords = match Coordinates::try_from(rect) {
            Ok(v) => v,
            Err(_) => return None,
        };
        let width = self.width() as isize;
        let height = self.height() as isize;
        let stride = self.stride();

        if coords.left < 0
            || coords.left >= width
            || coords.right > width
            || coords.top < 0
            || coords.top >= height
            || coords.bottom > height
        {
            return None;
        }

        let offset = rect.x() as usize + rect.y() as usize * stride;
        let new_len = rect.height() as usize * stride;
        let r = {
            let slice = self.slice_mut();
            let mut view = Bitmap8 {
                width: rect.width() as usize,
                height: rect.height() as usize,
                stride,
                slice: UnsafeCell::new(&mut slice[offset..offset + new_len]),
            };
            f(&mut view)
        };
        Some(r)
    }
}

impl Drawable for Bitmap8<'_> {
    type ColorType = IndexedColor;

    fn width(&self) -> usize {
        self.width
    }

    fn height(&self) -> usize {
        self.height
    }
}

impl RasterImage for Bitmap8<'_> {
    fn stride(&self) -> usize {
        self.stride
    }

    fn slice(&self) -> &[Self::ColorType] {
        unsafe { self.slice.get().as_ref().unwrap() }
    }
}

impl MutableRasterImage for Bitmap8<'_> {
    fn slice_mut(&mut self) -> &mut [Self::ColorType] {
        self.slice.get_mut()
    }
}

impl BasicDrawing for Bitmap8<'_> {
    fn fill_rect(&mut self, rect: Rect, color: Self::ColorType) {
        let mut width = rect.width();
        let mut height = rect.height();
        let mut dx = rect.x();
        let mut dy = rect.y();

        if dx < 0 {
            width += dx;
            dx = 0;
        }
        if dy < 0 {
            height += dy;
            dy = 0;
        }
        let r = dx + width;
        let b = dy + height;
        if r >= self.width as isize {
            width = self.width as isize - dx;
        }
        if b >= self.height as isize {
            height = self.height as isize - dy;
        }
        if width <= 0 || height <= 0 {
            return;
        }

        let width = width as usize;
        let height = height as usize;
        let stride = self.stride;
        let mut cursor = dx as usize + dy as usize * stride;
        if stride == width {
            memset_colors8(self.slice_mut(), cursor, width * height, color);
        } else {
            for _ in 0..height {
                memset_colors8(self.slice_mut(), cursor, width, color);
                cursor += stride;
            }
        }
    }

    fn draw_hline(&mut self, origin: Point, width: isize, color: Self::ColorType) {
        let mut dx = origin.x;
        let dy = origin.y;
        let mut w = width;

        if dy < 0 || dy >= (self.height as isize) {
            return;
        }
        if dx < 0 {
            w += dx;
            dx = 0;
        }
        let r = dx + w;
        if r >= (self.width as isize) {
            w = (self.width as isize) - dx;
        }
        if w <= 0 {
            return;
        }

        let cursor = dx as usize + dy as usize * self.stride;
        memset_colors8(self.slice_mut(), cursor, w as usize, color);
    }

    fn draw_vline(&mut self, origin: Point, height: isize, color: Self::ColorType) {
        let dx = origin.x;
        let mut dy = origin.y;
        let mut h = height;

        if dx < 0 || dx >= (self.width as isize) {
            return;
        }
        if dy < 0 {
            h += dy;
            dy = 0;
        }
        let b = dy + h;
        if b >= (self.height as isize) {
            h = (self.height as isize) - dy;
        }
        if h <= 0 {
            return;
        }

        let stride = self.stride;
        let mut cursor = dx as usize + dy as usize * stride;
        for _ in 0..h {
            self.slice_mut()[cursor] = color;
            cursor += stride;
        }
    }
}

impl RasterFontWriter for Bitmap8<'_> {}

impl<'a> From<&'a Bitmap8<'a>> for ConstBitmap8<'a> {
    fn from(src: &'a Bitmap8<'a>) -> ConstBitmap8<'a> {
        ConstBitmap8::from_slice(src.slice(), src.size(), src.stride())
    }
}

#[repr(C)]
pub struct VecBitmap8 {
    width: usize,
    height: usize,
    stride: usize,
    vec: Vec<IndexedColor>,
}

impl VecBitmap8 {
    pub fn new(size: Size, bg_color: IndexedColor) -> Self {
        let len = size.width() as usize * size.height() as usize;
        let mut vec = Vec::with_capacity(len);
        vec.resize_with(len, || bg_color);
        Self {
            width: size.width() as usize,
            height: size.height() as usize,
            stride: size.width() as usize,
            vec,
        }
    }
}

impl Drawable for VecBitmap8 {
    type ColorType = IndexedColor;

    fn width(&self) -> usize {
        self.width
    }

    fn height(&self) -> usize {
        self.height
    }
}

impl RasterImage for VecBitmap8 {
    fn stride(&self) -> usize {
        self.stride
    }

    fn slice(&self) -> &[Self::ColorType] {
        self.vec.as_slice()
    }
}

impl MutableRasterImage for VecBitmap8 {
    fn slice_mut(&mut self) -> &mut [Self::ColorType] {
        self.vec.as_mut_slice()
    }
}

impl<'a> From<&'a VecBitmap8> for ConstBitmap8<'a> {
    fn from(src: &'a VecBitmap8) -> Self {
        let size = src.size();
        let stride = src.stride();
        Self::from_slice(src.slice(), size, stride)
    }
}

impl<'a> From<&'a mut VecBitmap8> for Bitmap8<'a> {
    fn from(src: &'a mut VecBitmap8) -> Self {
        let size = src.size();
        let stride = src.stride();
        Self::from_slice(src.slice_mut(), size, stride)
    }
}

/// Fast fill
#[inline]
fn memset_colors8(slice: &mut [IndexedColor], cursor: usize, size: usize, color: IndexedColor) {
    // let slice = &mut slice[cursor..cursor + size];
    unsafe {
        let slice = slice.get_unchecked_mut(cursor);
        let color = color.0;
        let mut ptr: *mut u8 = transmute(slice);
        let mut remain = size;

        let prologue = usize::min(ptr as usize & 0x0F, remain);
        remain -= prologue;
        for _ in 0..prologue {
            ptr.write_volatile(color);
            ptr = ptr.add(1);
        }

        if remain > 16 {
            let color32 =
                color as u32 | (color as u32) << 8 | (color as u32) << 16 | (color as u32) << 24;
            let color64 = color32 as u64 | (color32 as u64) << 32;
            let color128 = color64 as u128 | (color64 as u128) << 64;
            let count = remain / 16;
            let mut ptr2 = ptr as *mut u128;

            for _ in 0..count {
                ptr2.write_volatile(color128);
                ptr2 = ptr2.add(1);
            }

            ptr = ptr2 as *mut u8;
            remain -= count * 16;
        }

        for _ in 0..remain {
            ptr.write_volatile(color);
            ptr = ptr.add(1);
        }
    }
}

/// Fast copy
#[inline]
fn memcpy_colors8(
    dest: &mut [IndexedColor],
    dest_cursor: usize,
    src: &[IndexedColor],
    src_cursor: usize,
    size: usize,
) {
    // let dest = &mut dest[dest_cursor..dest_cursor + size];
    // let src = &src[src_cursor..src_cursor + size];
    unsafe {
        let dest = dest.get_unchecked_mut(dest_cursor);
        let src = src.get_unchecked(src_cursor);
        let mut ptr_d: *mut u8 = transmute(dest);
        let mut ptr_s: *const u8 = transmute(src);
        let mut remain = size;

        if ((ptr_d as usize) & 0x7) == ((ptr_s as usize) & 0x7) {
            let prologue = usize::min(ptr_d as usize & 0x07, remain);
            remain -= prologue;
            for _ in 0..prologue {
                ptr_d.write_volatile(ptr_s.read_volatile());
                ptr_d = ptr_d.add(1);
                ptr_s = ptr_s.add(1);
            }

            if remain > 8 {
                let count = remain / 8;
                let mut ptr2d = ptr_d as *mut u64;
                let mut ptr2s = ptr_s as *const u64;

                for _ in 0..count {
                    ptr2d.write_volatile(ptr2s.read_volatile());
                    ptr2d = ptr2d.add(1);
                    ptr2s = ptr2s.add(1);
                }

                ptr_d = ptr2d as *mut u8;
                ptr_s = ptr2s as *const u8;
                remain -= count * 8;
            }

            for _ in 0..remain {
                ptr_d.write_volatile(ptr_s.read_volatile());
                ptr_d = ptr_d.add(1);
                ptr_s = ptr_s.add(1);
            }
        } else {
            for _ in 0..size {
                ptr_d.write_volatile(ptr_s.read_volatile());
                ptr_d = ptr_d.add(1);
                ptr_s = ptr_s.add(1);
            }
        }
    }
}

//-//

#[repr(C)]
pub struct ConstBitmap32<'a> {
    width: usize,
    height: usize,
    stride: usize,
    slice: &'a [TrueColor],
}

bitflags! {
    pub struct BitmapFlags: usize {
        const PORTRAIT      = 0b0000_0001;
        const TRANSLUCENT   = 0b0000_0010;
        const VIEW          = 0b1000_0000;
    }
}

impl<'a> ConstBitmap32<'a> {
    #[inline]
    pub const fn from_slice(slice: &'a [TrueColor], size: Size, stride: usize) -> Self {
        Self {
            width: size.width() as usize,
            height: size.height() as usize,
            stride,
            slice,
        }
    }

    #[inline]
    pub const fn from_bytes(bytes: &'a [u32], size: Size) -> Self {
        Self {
            width: size.width() as usize,
            height: size.height() as usize,
            stride: size.width() as usize,
            slice: unsafe { transmute(bytes) },
        }
    }

    #[inline]
    pub fn clone(&'a self) -> Self {
        Self {
            width: self.width(),
            height: self.height(),
            stride: self.stride(),
            slice: self.slice(),
        }
    }
}

impl Drawable for ConstBitmap32<'_> {
    type ColorType = TrueColor;

    fn width(&self) -> usize {
        self.width
    }

    fn height(&self) -> usize {
        self.height
    }
}

impl RasterImage for ConstBitmap32<'_> {
    fn stride(&self) -> usize {
        self.stride
    }

    fn slice(&self) -> &[Self::ColorType] {
        self.slice
    }
}

#[repr(C)]
pub struct Bitmap32<'a> {
    width: usize,
    height: usize,
    stride: usize,
    slice: UnsafeCell<&'a mut [TrueColor]>,
}

impl<'a> Bitmap32<'a> {
    #[inline]
    pub fn from_slice(slice: &'a mut [TrueColor], size: Size, stride: usize) -> Self {
        Self {
            width: size.width() as usize,
            height: size.height() as usize,
            stride,
            slice: UnsafeCell::new(slice),
        }
    }

    /// Clone a bitmap
    #[inline]
    pub fn clone(&self) -> Bitmap32<'a> {
        let slice = unsafe { self.slice.get().as_mut().unwrap() };
        Self {
            width: self.width(),
            height: self.height(),
            stride: self.stride(),
            slice: UnsafeCell::new(slice),
        }
    }
}

impl Bitmap32<'_> {
    pub fn blend_rect(&mut self, rect: Rect, color: TrueColor) {
        let rhs = color.components();
        if rhs.is_opaque() {
            return self.fill_rect(rect, color);
        } else if rhs.is_transparent() {
            return;
        }
        let alpha = rhs.a as usize;
        let alpha_n = 255 - alpha;

        let mut width = rect.size.width;
        let mut height = rect.size.height;
        let mut dx = rect.origin.x;
        let mut dy = rect.origin.y;

        if dx < 0 {
            width += dx;
            dx = 0;
        }
        if dy < 0 {
            height += dy;
            dy = 0;
        }
        let r = dx + width;
        let b = dy + height;
        if r >= self.size().width {
            width = self.size().width - dx;
        }
        if b >= self.size().height {
            height = self.size().height - dy;
        }
        if width <= 0 || height <= 0 {
            return;
        }

        // if self.is_portrait() {
        //     let temp = dx;
        //     dx = self.size().height - dy - height;
        //     dy = temp;
        //     swap(&mut width, &mut height);
        // }

        let mut cursor = dx as usize + dy as usize * self.stride();
        let stride = self.stride() - width as usize;
        let slice = self.slice_mut();
        for _ in 0..height {
            for _ in 0..width {
                let lhs = unsafe { slice.get_unchecked(cursor) }.components();
                let c = lhs
                    .blend_color(
                        rhs,
                        |lhs, rhs| {
                            (((lhs as usize) * alpha_n + (rhs as usize) * alpha) / 255) as u8
                        },
                        |a, b| a.saturating_add(b),
                    )
                    .into();
                unsafe {
                    *slice.get_unchecked_mut(cursor) = c;
                }
                cursor += 1;
            }
            cursor += stride;
        }
    }
}

impl Bitmap32<'static> {
    /// SAFETY: Must guarantee the existence of the `ptr`.
    #[inline]
    pub unsafe fn from_static(ptr: *mut TrueColor, size: Size, stride: usize) -> Self {
        let slice = core::slice::from_raw_parts_mut(ptr, size.height() as usize * stride);
        Self {
            width: size.width() as usize,
            height: size.height() as usize,
            stride,
            slice: UnsafeCell::new(slice),
        }
    }
}

impl Drawable for Bitmap32<'_> {
    type ColorType = TrueColor;

    fn width(&self) -> usize {
        self.width
    }

    fn height(&self) -> usize {
        self.height
    }
}

impl RasterImage for Bitmap32<'_> {
    fn stride(&self) -> usize {
        self.stride
    }

    fn slice(&self) -> &[Self::ColorType] {
        unsafe { self.slice.get().as_ref().unwrap() }
    }
}

impl MutableRasterImage for Bitmap32<'_> {
    fn slice_mut(&mut self) -> &mut [Self::ColorType] {
        self.slice.get_mut()
    }
}

impl BasicDrawing for Bitmap32<'_> {
    fn fill_rect(&mut self, rect: Rect, color: Self::ColorType) {
        let mut width = rect.width();
        let mut height = rect.height();
        let mut dx = rect.x();
        let mut dy = rect.y();

        if dx < 0 {
            width += dx;
            dx = 0;
        }
        if dy < 0 {
            height += dy;
            dy = 0;
        }
        let r = dx + width;
        let b = dy + height;
        if r >= self.width as isize {
            width = self.width as isize - dx;
        }
        if b >= self.height as isize {
            height = self.height as isize - dy;
        }
        if width <= 0 || height <= 0 {
            return;
        }

        let width = width as usize;
        let height = height as usize;
        let stride = self.stride;
        let mut cursor = dx as usize + dy as usize * stride;
        if stride == width {
            memset_colors32(self.slice_mut(), cursor, width * height, color);
        } else {
            for _ in 0..height {
                memset_colors32(self.slice_mut(), cursor, width, color);
                cursor += stride;
            }
        }
    }

    fn draw_hline(&mut self, origin: Point, width: isize, color: Self::ColorType) {
        let mut dx = origin.x;
        let dy = origin.y;
        let mut w = width;

        if dy < 0 || dy >= (self.height as isize) {
            return;
        }
        if dx < 0 {
            w += dx;
            dx = 0;
        }
        let r = dx + w;
        if r >= (self.width as isize) {
            w = (self.width as isize) - dx;
        }
        if w <= 0 {
            return;
        }

        let cursor = dx as usize + dy as usize * self.stride;
        memset_colors32(self.slice_mut(), cursor, w as usize, color);
    }

    fn draw_vline(&mut self, origin: Point, height: isize, color: Self::ColorType) {
        let dx = origin.x;
        let mut dy = origin.y;
        let mut h = height;

        if dx < 0 || dx >= (self.width as isize) {
            return;
        }
        if dy < 0 {
            h += dy;
            dy = 0;
        }
        let b = dy + h;
        if b >= (self.height as isize) {
            h = (self.height as isize) - dy;
        }
        if h <= 0 {
            return;
        }

        let stride = self.stride;
        let mut cursor = dx as usize + dy as usize * stride;
        for _ in 0..h {
            self.slice_mut()[cursor] = color;
            cursor += stride;
        }
    }
}

impl RasterFontWriter for Bitmap32<'_> {}

impl<'a> From<&'a Bitmap32<'a>> for ConstBitmap32<'a> {
    fn from(src: &'a Bitmap32<'a>) -> Self {
        Self::from_slice(src.slice(), src.size(), src.stride())
    }
}

impl BitmapDrawing32 for Bitmap32<'_> {}

pub enum BltMode {
    Blend,
    Copy,
}

pub trait BitmapDrawing32: MutableRasterImage<ColorType = TrueColor> {
    fn blt<T>(&mut self, src: &T, origin: Point, rect: Rect)
    where
        T: RasterImage<ColorType = <Self as Drawable>::ColorType>,
    {
        self.blt_main(src, origin, rect, BltMode::Copy);
    }

    #[inline]
    fn blt_main<T>(&mut self, src: &T, origin: Point, rect: Rect, mode: BltMode)
    where
        T: RasterImage<ColorType = <Self as Drawable>::ColorType>,
    {
        let mut dx = origin.x;
        let mut dy = origin.y;
        let mut sx = rect.origin.x;
        let mut sy = rect.origin.y;
        let mut width = rect.width();
        let mut height = rect.height();

        if dx < 0 {
            sx -= dx;
            width += dx;
            dx = 0;
        }
        if dy < 0 {
            sy -= dy;
            height += dy;
            dy = 0;
        }
        let sw = src.width() as isize;
        let sh = src.height() as isize;
        if width > sx + sw {
            width = sw - sx;
        }
        if height > sy + sh {
            height = sh - sy;
        }
        let r = dx + width;
        let b = dy + height;
        let dw = self.width() as isize;
        let dh = self.height() as isize;
        if r >= dw {
            width = dw - dx;
        }
        if b >= dh {
            height = dh - dy;
        }
        if width <= 0 || height <= 0 {
            return;
        }

        let width = width as usize;
        let height = height as usize;

        let ds = self.stride();
        let ss = src.stride();
        let mut dest_cursor = dx as usize + dy as usize * ds;
        let mut src_cursor = sx as usize + sy as usize * ss;
        let dest_fb = self.slice_mut();
        let src_fb = src.slice();

        match mode {
            BltMode::Copy => {
                if ds == width && ss == width {
                    memcpy_colors32(dest_fb, dest_cursor, src_fb, src_cursor, width * height);
                } else {
                    for _ in 0..height {
                        memcpy_colors32(dest_fb, dest_cursor, src_fb, src_cursor, width);
                        dest_cursor += ds;
                        src_cursor += ss;
                    }
                }
            }
            _ => {
                for _ in 0..height {
                    blend_line32(dest_fb, dest_cursor, src_fb, src_cursor, width);
                    dest_cursor += ds;
                    src_cursor += ss;
                }
            }
        }
    }

    fn translate<T>(&mut self, src: &T, origin: Point, rect: Rect, palette: &[u32; 256])
    where
        T: RasterImage<ColorType = IndexedColor>,
    {
        let mut dx = origin.x;
        let mut dy = origin.y;
        let mut sx = rect.origin.x;
        let mut sy = rect.origin.y;
        let mut width = rect.width();
        let mut height = rect.height();

        if dx < 0 {
            sx -= dx;
            width += dx;
            dx = 0;
        }
        if dy < 0 {
            sy -= dy;
            height += dy;
            dy = 0;
        }
        let sw = src.width() as isize;
        let sh = src.height() as isize;
        if width > sx + sw {
            width = sw - sx;
        }
        if height > sy + sh {
            height = sh - sy;
        }
        let r = dx + width;
        let b = dy + height;
        let dw = self.width() as isize;
        let dh = self.height() as isize;
        if r >= dw {
            width = dw - dx;
        }
        if b >= dh {
            height = dh - dy;
        }
        if width <= 0 || height <= 0 {
            return;
        }

        let width = width as usize;
        let height = height as usize;

        let ds = self.stride();
        let ss = src.stride();
        let mut dest_cursor = dx as usize + dy as usize * ds;
        let mut src_cursor = sx as usize + sy as usize * ss;
        let dest_fb = self.slice_mut();
        let src_fb = src.slice();

        let dd = ds - width;
        let sd = ss - width;
        for _ in 0..height {
            for _ in 0..width {
                let c8 = src_fb[src_cursor].0 as usize;
                dest_fb[dest_cursor] = TrueColor::from_argb(palette[c8]);
                src_cursor += 1;
                dest_cursor += 1;
            }
            dest_cursor += dd;
            src_cursor += sd;
        }
    }

    /// Make a bitmap view
    fn view<'a, F, R>(&'a mut self, rect: Rect, f: F) -> Option<R>
    where
        F: FnOnce(&mut Bitmap32) -> R,
    {
        let coords = match Coordinates::try_from(rect) {
            Ok(v) => v,
            Err(_) => return None,
        };
        let width = self.width() as isize;
        let height = self.height() as isize;
        let stride = self.stride();

        if coords.left < 0
            || coords.left >= width
            || coords.right > width
            || coords.top < 0
            || coords.top >= height
            || coords.bottom > height
        {
            return None;
        }

        let offset = rect.x() as usize + rect.y() as usize * stride;
        let new_len = rect.height() as usize * stride;
        let r = {
            let slice = self.slice_mut();
            let mut view = Bitmap32 {
                width: rect.width() as usize,
                height: rect.height() as usize,
                stride,
                slice: UnsafeCell::new(&mut slice[offset..offset + new_len]),
            };
            f(&mut view)
        };
        Some(r)
    }
}

#[repr(C)]
pub struct VecBitmap32 {
    width: usize,
    height: usize,
    stride: usize,
    vec: Vec<TrueColor>,
}

impl VecBitmap32 {
    pub fn new(size: Size, bg_color: TrueColor) -> Self {
        let len = size.width() as usize * size.height() as usize;
        let mut vec = Vec::with_capacity(len);
        vec.resize_with(len, || bg_color);
        Self {
            width: size.width() as usize,
            height: size.height() as usize,
            stride: size.width() as usize,
            vec,
        }
    }
}

impl Drawable for VecBitmap32 {
    type ColorType = TrueColor;

    fn width(&self) -> usize {
        self.width
    }

    fn height(&self) -> usize {
        self.height
    }
}

impl RasterImage for VecBitmap32 {
    fn stride(&self) -> usize {
        self.stride
    }

    fn slice(&self) -> &[Self::ColorType] {
        self.vec.as_slice()
    }
}

impl MutableRasterImage for VecBitmap32 {
    fn slice_mut(&mut self) -> &mut [Self::ColorType] {
        self.vec.as_mut_slice()
    }
}

impl<'a> From<&'a VecBitmap32> for ConstBitmap32<'a> {
    fn from(src: &'a VecBitmap32) -> Self {
        let size = src.size();
        let stride = src.stride();
        Self::from_slice(src.slice(), size, stride)
    }
}

impl<'a> From<&'a mut VecBitmap32> for Bitmap32<'a> {
    fn from(src: &'a mut VecBitmap32) -> Self {
        let size = src.size();
        let stride = src.stride();
        Self::from_slice(src.slice_mut(), size, stride)
    }
}

/// Fast Fill
#[inline]
fn memset_colors32(slice: &mut [TrueColor], cursor: usize, count: usize, color: TrueColor) {
    let slice = &mut slice[cursor..cursor + count];
    unsafe {
        let color32 = color.argb();
        let mut ptr: *mut u32 = core::mem::transmute(&slice[0]);
        let mut remain = count;

        let prologue = usize::min(ptr as usize & 0x0F / 4, remain);
        remain -= prologue;
        for _ in 0..prologue {
            ptr.write_volatile(color32);
            ptr = ptr.add(1);
        }

        if remain > 4 {
            let color128 = color32 as u128
                | (color32 as u128) << 32
                | (color32 as u128) << 64
                | (color32 as u128) << 96;
            let count = remain / 4;
            let mut ptr2 = ptr as *mut u128;

            for _ in 0..count {
                ptr2.write_volatile(color128);
                ptr2 = ptr2.add(1);
            }

            ptr = ptr2 as *mut u32;
            remain -= count * 4;
        }

        for _ in 0..remain {
            ptr.write_volatile(color32);
            ptr = ptr.add(1);
        }
    }
}

/// Fast copy
#[inline]
fn memcpy_colors32(
    dest: &mut [TrueColor],
    dest_cursor: usize,
    src: &[TrueColor],
    src_cursor: usize,
    count: usize,
) {
    let dest = &mut dest[dest_cursor..dest_cursor + count];
    let src = &src[src_cursor..src_cursor + count];
    unsafe {
        let mut ptr_d: *mut u32 = core::mem::transmute(&dest[0]);
        let mut ptr_s: *const u32 = core::mem::transmute(&src[0]);
        let mut remain = count;
        if ((ptr_d as usize) & 0xF) == ((ptr_s as usize) & 0xF) {
            let prologue = usize::min(ptr_d as usize & 0x0F, remain);
            remain -= prologue;
            for _ in 0..prologue {
                ptr_d.write_volatile(ptr_s.read_volatile());
                ptr_d = ptr_d.add(1);
                ptr_s = ptr_s.add(1);
            }

            if remain > 4 {
                let count = remain / 4;
                let mut ptr2d = ptr_d as *mut u128;
                let mut ptr2s = ptr_s as *mut u128;

                for _ in 0..count {
                    ptr2d.write_volatile(ptr2s.read_volatile());
                    ptr2d = ptr2d.add(1);
                    ptr2s = ptr2s.add(1);
                }

                ptr_d = ptr2d as *mut u32;
                ptr_s = ptr2s as *mut u32;
                remain -= count * 4;
            }

            for _ in 0..remain {
                ptr_d.write_volatile(ptr_s.read_volatile());
                ptr_d = ptr_d.add(1);
                ptr_s = ptr_s.add(1);
            }
        } else {
            for i in 0..count {
                dest[i] = src[i];
            }
        }
    }
}

#[inline]
fn blend_line32(
    dest: &mut [TrueColor],
    dest_cursor: usize,
    src: &[TrueColor],
    src_cursor: usize,
    count: usize,
) {
    let dest = &mut dest[dest_cursor..dest_cursor + count];
    let src = &src[src_cursor..src_cursor + count];
    for i in 0..count {
        dest[i] = dest[i].blend(src[i]);
    }
}

//-//

pub enum ConstBitmap<'a> {
    Indexed(ConstBitmap8<'a>),
    Argb32(ConstBitmap32<'a>),
}

impl Drawable for ConstBitmap<'_> {
    type ColorType = AmbiguousColor;

    #[inline]
    fn width(&self) -> usize {
        match self {
            Self::Indexed(v) => v.width(),
            Self::Argb32(v) => v.width(),
        }
    }

    #[inline]
    fn height(&self) -> usize {
        match self {
            Self::Indexed(v) => v.height(),
            Self::Argb32(v) => v.height(),
        }
    }
}

impl<'a> From<ConstBitmap8<'a>> for ConstBitmap<'a> {
    #[inline]
    fn from(val: ConstBitmap8<'a>) -> ConstBitmap<'a> {
        ConstBitmap::Indexed(val)
    }
}

impl<'a> From<ConstBitmap32<'a>> for ConstBitmap<'a> {
    #[inline]
    fn from(val: ConstBitmap32<'a>) -> ConstBitmap {
        ConstBitmap::Argb32(val)
    }
}

impl<'a> From<&'a Bitmap8<'a>> for ConstBitmap<'a> {
    #[inline]
    fn from(val: &'a Bitmap8<'a>) -> ConstBitmap {
        ConstBitmap::Indexed(val.into())
    }
}

impl<'a> From<&'a Bitmap32<'a>> for ConstBitmap<'a> {
    #[inline]
    fn from(val: &'a Bitmap32<'a>) -> ConstBitmap {
        ConstBitmap::Argb32(val.into())
    }
}

pub enum Bitmap<'a> {
    Indexed(Bitmap8<'a>),
    Argb32(Bitmap32<'a>),
}

impl Drawable for Bitmap<'_> {
    type ColorType = AmbiguousColor;

    #[inline]
    fn width(&self) -> usize {
        match self {
            Self::Indexed(v) => v.width(),
            Self::Argb32(v) => v.width(),
        }
    }

    #[inline]
    fn height(&self) -> usize {
        match self {
            Self::Indexed(v) => v.height(),
            Self::Argb32(v) => v.height(),
        }
    }
}

impl GetPixel for Bitmap<'_> {
    #[inline]
    unsafe fn get_pixel_unchecked(&self, point: Point) -> Self::ColorType {
        match self {
            Bitmap::Indexed(v) => v.get_pixel_unchecked(point).into(),
            Bitmap::Argb32(v) => v.get_pixel_unchecked(point).into(),
        }
    }
}

impl SetPixel for Bitmap<'_> {
    #[inline]
    unsafe fn set_pixel_unchecked(&mut self, point: Point, pixel: Self::ColorType) {
        match self {
            Bitmap::Indexed(v) => v.set_pixel_unchecked(point, pixel.into()),
            Bitmap::Argb32(v) => v.set_pixel_unchecked(point, pixel.into()),
        }
    }
}

impl RasterFontWriter for Bitmap<'_> {
    #[inline]
    fn draw_font(&mut self, src: &[u8], size: Size, origin: Point, color: Self::ColorType) {
        match self {
            Bitmap::Indexed(v) => v.draw_font(src, size, origin, color.into()),
            Bitmap::Argb32(v) => v.draw_font(src, size, origin, color.into()),
        }
    }
}

impl BasicDrawing for Bitmap<'_> {
    #[inline]
    fn fill_rect(&mut self, rect: Rect, color: Self::ColorType) {
        match self {
            Bitmap::Indexed(v) => v.fill_rect(rect, color.into()),
            Bitmap::Argb32(v) => v.fill_rect(rect, color.into()),
        }
    }

    #[inline]
    fn draw_hline(&mut self, origin: Point, width: isize, color: Self::ColorType) {
        match self {
            Bitmap::Indexed(v) => v.draw_hline(origin, width, color.into()),
            Bitmap::Argb32(v) => v.draw_hline(origin, width, color.into()),
        }
    }

    #[inline]
    fn draw_vline(&mut self, origin: Point, height: isize, color: Self::ColorType) {
        match self {
            Bitmap::Indexed(v) => v.draw_vline(origin, height, color.into()),
            Bitmap::Argb32(v) => v.draw_vline(origin, height, color.into()),
        }
    }
}

impl<'a> Bitmap<'a> {
    #[inline]
    pub fn clone(&self) -> Bitmap<'a> {
        match self {
            Bitmap::Indexed(v) => Self::from(v.clone()),
            Bitmap::Argb32(v) => Self::from(v.clone()),
        }
    }
}

impl Bitmap<'_> {
    #[inline]
    pub fn blt_itself(&mut self, origin: Point, rect: Rect) {
        match self {
            Bitmap::Indexed(v) => v.blt(&v.clone(), origin, rect),
            Bitmap::Argb32(v) => v.blt(&v.clone(), origin, rect),
        }
    }
}

impl Blt<ConstBitmap<'_>> for Bitmap<'_> {
    fn blt(&mut self, src: &ConstBitmap<'_>, origin: Point, rect: Rect) {
        match self {
            Bitmap::Indexed(bitmap) => match src {
                ConstBitmap::Indexed(src) => bitmap.blt(src, origin, rect),
                ConstBitmap::Argb32(_src) => todo!(),
            },
            Bitmap::Argb32(bitmap) => match src {
                ConstBitmap::Indexed(src) => {
                    bitmap.translate(src, origin, rect, &IndexedColor::COLOR_PALETTE)
                }
                ConstBitmap::Argb32(src) => bitmap.blt(src, origin, rect),
            },
        }
    }
}

impl Blt<Bitmap<'_>> for Bitmap<'_> {
    fn blt(&mut self, src: &Bitmap<'_>, origin: Point, rect: Rect) {
        match self {
            Bitmap::Indexed(bitmap) => match src {
                Bitmap::Indexed(src) => bitmap.blt(src.into(), origin, rect),
                Bitmap::Argb32(_src) => todo!(),
            },
            Bitmap::Argb32(bitmap) => match src {
                Bitmap::Indexed(src) => {
                    bitmap.translate(src, origin, rect, &IndexedColor::COLOR_PALETTE)
                }
                Bitmap::Argb32(src) => bitmap.blt(src.into(), origin, rect),
            },
        }
    }
}

impl Blt<ConstBitmap8<'_>> for Bitmap<'_> {
    fn blt(&mut self, src: &ConstBitmap8<'_>, origin: Point, rect: Rect) {
        match self {
            Bitmap::Indexed(bitmap) => bitmap.blt(src, origin, rect),
            Bitmap::Argb32(bitmap) => {
                bitmap.translate(src, origin, rect, &IndexedColor::COLOR_PALETTE)
            }
        }
    }
}

impl Blt<ConstBitmap32<'_>> for Bitmap<'_> {
    fn blt(&mut self, src: &ConstBitmap32<'_>, origin: Point, rect: Rect) {
        match self {
            Bitmap::Indexed(_bitmap) => todo!(),
            Bitmap::Argb32(bitmap) => bitmap.blt(src, origin, rect),
        }
    }
}

impl<'a> From<Bitmap8<'a>> for Bitmap<'a> {
    #[inline]
    fn from(val: Bitmap8<'a>) -> Self {
        Self::Indexed(val)
    }
}

impl<'a> From<Bitmap32<'a>> for Bitmap<'a> {
    #[inline]
    fn from(val: Bitmap32<'a>) -> Self {
        Self::Argb32(val)
    }
}

pub enum VecBitmap {
    Indexed(VecBitmap8),
    Argb32(VecBitmap32),
}

impl Drawable for VecBitmap {
    type ColorType = AmbiguousColor;

    #[inline]
    fn width(&self) -> usize {
        match self {
            Self::Indexed(v) => v.width(),
            Self::Argb32(v) => v.width(),
        }
    }

    #[inline]
    fn height(&self) -> usize {
        match self {
            Self::Indexed(v) => v.height(),
            Self::Argb32(v) => v.height(),
        }
    }
}

impl<'a> From<&'a mut VecBitmap> for Bitmap<'a> {
    #[inline]
    fn from(val: &'a mut VecBitmap) -> Self {
        match val {
            VecBitmap::Indexed(v) => Bitmap::Indexed(v.into()),
            VecBitmap::Argb32(v) => Bitmap::Argb32(v.into()),
        }
    }
}

impl From<VecBitmap8> for VecBitmap {
    #[inline]
    fn from(val: VecBitmap8) -> Self {
        Self::Indexed(val)
    }
}

impl From<VecBitmap32> for VecBitmap {
    #[inline]
    fn from(val: VecBitmap32) -> Self {
        Self::Argb32(val)
    }
}
