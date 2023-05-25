const std = @import("std");

pub extern "forward" fn read_args(dest: *u8) void;
pub extern "forward" fn return_data(data: *const u8, len: usize) void;

pub fn args(len: usize) ![]u8 {
    var input = try std.heap.page_allocator.alloc(u8, len);
    read_args(@ptrCast(*u8, input));
    return input;
}

pub fn output(data: []u8) void {
    return_data(@ptrCast(*u8, data), data.len);
}

/// Modifies the input's second argument intentionally to test things out.
pub fn tweak(input: []u8) []u8 {
    input[1] = 0x05;
    return input;
}

export fn arbitrum_main(len: usize) i32 {
    var input = args(len) catch return 1;
    var result = tweak(input);
    output(result);
    return 0; // OK.
}
