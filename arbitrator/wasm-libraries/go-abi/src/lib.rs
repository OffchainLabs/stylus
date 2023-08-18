// Copyright 2021-2023, Offchain Labs, Inc.
// For license information, see https://github.com/nitro/blob/master/LICENSE

use std::convert::TryInto;

use arbutil::wavm;

extern "C" {
    pub fn wavm_guest_call__getsp() -> usize;
    pub fn wavm_guest_call__resume();
}

#[derive(Clone)]
pub struct GoStack {
    sp: usize,
    top: usize,
}

impl GoStack {
    pub fn new(sp: usize) -> Self {
        let top = sp + 8;
        Self { sp, top }
    }

    /// returns the pointer at which a value may be accessed, moving the offset past the value
    fn advance(&mut self, bytes: usize) -> usize {
        let before = self.top;
        self.top += bytes;
        before
    }

    pub unsafe fn read_u8(&mut self) -> u8 {
        wavm::caller_load8(self.advance(1))
    }

    pub unsafe fn read_u16(&mut self) -> u16 {
        wavm::caller_load16(self.advance(2))
    }

    pub unsafe fn read_u32(&mut self) -> u32 {
        wavm::caller_load32(self.advance(4))
    }

    pub unsafe fn read_u64(&mut self) -> u64 {
        wavm::caller_load64(self.advance(8))
    }

    pub unsafe fn read_ptr<T>(&mut self) -> *const T {
        self.read_u64() as *const T
    }

    pub unsafe fn read_ptr_mut<T>(&mut self) -> *mut T {
        self.read_u64() as *mut T
    }

    pub unsafe fn unbox<T>(&mut self) -> T {
        *Box::from_raw(self.read_ptr_mut())
    }

    pub unsafe fn unbox_option<T>(&mut self) -> Option<T> {
        let ptr: *mut T = self.read_ptr_mut();
        (!ptr.is_null()).then(|| *Box::from_raw(ptr))
    }

    pub unsafe fn read_bool32(&mut self) -> bool {
        self.read_u32() != 0
    }

    pub unsafe fn read_go_ptr(&mut self) -> usize {
        self.read_u64().try_into().expect("go pointer doesn't fit")
    }

    pub unsafe fn write_u8(&mut self, x: u8) -> &mut Self {
        wavm::caller_store8(self.advance(1), x);
        self
    }

    pub unsafe fn write_u16(&mut self, x: u16) -> &mut Self {
        wavm::caller_store16(self.advance(2), x);
        self
    }

    pub unsafe fn write_u32(&mut self, x: u32) -> &mut Self {
        wavm::caller_store32(self.advance(4), x);
        self
    }

    pub unsafe fn write_u64(&mut self, x: u64) -> &mut Self {
        wavm::caller_store64(self.advance(8), x);
        self
    }

    pub unsafe fn write_ptr<T>(&mut self, ptr: *const T) -> &mut Self {
        self.write_u64(ptr as u64)
    }

    pub unsafe fn write_nullptr(&mut self) -> &mut Self {
        self.write_ptr(std::ptr::null::<u8>())
    }

    pub fn skip_u8(&mut self) -> &mut Self {
        self.advance(1);
        self
    }

    pub fn skip_u16(&mut self) -> &mut Self {
        self.advance(2);
        self
    }

    pub fn skip_u32(&mut self) -> &mut Self {
        self.advance(4);
        self
    }

    pub fn skip_u64(&mut self) -> &mut Self {
        self.advance(8);
        self
    }

    /// skips the rest of the remaining space in a u64
    pub fn skip_space(&mut self) -> &mut Self {
        self.advance(8 - (self.top - self.sp) % 8);
        self
    }

    pub unsafe fn read_go_slice(&mut self) -> (u64, u64) {
        let ptr = self.read_u64();
        let len = self.read_u64();
        self.skip_u64(); // skip the slice's capacity
        (ptr, len)
    }

    pub unsafe fn read_go_slice_owned(&mut self) -> Vec<u8> {
        let (ptr, len) = self.read_go_slice();
        wavm::read_slice(ptr, len)
    }

    pub unsafe fn read_js_string(&mut self) -> Vec<u8> {
        let ptr = self.read_u64();
        let len = self.read_u64();
        wavm::read_slice(ptr, len)
    }

    /// Saves the stack pointer for later calls to `restore_stack`.
    pub fn save_stack(&self) -> usize {
        self.top - (self.sp + 8)
    }

    /// Writes the stack pointer.
    ///
    /// # Safety
    ///
    /// `saved` must come from `save_stack`, with no calls to `advance` in between.
    pub unsafe fn restore_stack(&mut self, saved: usize) {
        *self = Self::new(wavm_guest_call__getsp());
        self.advance(saved);
    }

    /// Resumes the go runtime, updating the stack pointer.
    ///
    /// # Safety
    ///
    /// The caller must cut lifetimes before this call.
    pub unsafe fn resume(&mut self) {
        let saved = self.save_stack();
        wavm_guest_call__resume();
        self.restore_stack(saved);
    }
}

#[test]
fn test_sp() {
    let mut sp = GoStack::new(0);
    assert_eq!(sp.advance(3), 8 + 0);
    assert_eq!(sp.advance(2), 8 + 3);
    assert_eq!(sp.skip_space().top, 8 + 8);
    assert_eq!(sp.skip_space().top, 8 + 16);
    assert_eq!(sp.skip_u32().skip_space().top, 8 + 24);
}
