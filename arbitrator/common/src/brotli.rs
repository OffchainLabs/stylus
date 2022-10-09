// Copyright 2022, Offchain Labs, Inc.
// For license information, see https://github.com/nitro/blob/master/LICENSE

extern "C" {
    pub fn BrotliDecoderDecompress(
        encoded_size: usize,
        encoded_buffer: *const u8,
        decoded_size: *mut usize,
        decoded_buffer: *mut u8,
    ) -> u32;

    pub fn BrotliEncoderCompress(
        quality: u32,
        lgwin: u32,
        mode: u32,
        input_size: usize,
        input_buffer: *const u8,
        encoded_size: *mut usize,
        encoded_buffer: *mut u8,
    ) -> u32;

    pub fn BrotliEncoderMaxCompressedSize(input_size: usize) -> usize;
}

const BROTLI_MODE_GENERIC: u32 = 0;
const BROTLI_RES_SUCCESS: u32 = 1;

pub fn compress(data: &[u8], quality: u32, window: u32) -> Result<Vec<u8>, String> {
    unsafe {
        let needed = BrotliEncoderMaxCompressedSize(data.len());
        let mut buffer = Vec::with_capacity(needed);

        let mut size = buffer.capacity();
        let status = BrotliEncoderCompress(
            quality,
            window,
            BROTLI_MODE_GENERIC,
            data.len(),
            data.as_ptr(),
            &mut size,
            buffer.as_mut_ptr(),
        );
        if status != BROTLI_RES_SUCCESS {
            return Err(format!("failed to compress data {status}"));
        }

        buffer.set_len(size);
        Ok(buffer)
    }
}

pub fn decompress(data: &[u8]) -> Result<Vec<u8>, String> {
    unsafe {
        let mut buffer = Vec::with_capacity(data.len() + 64 + 1024 * 1024);
        let mut size = buffer.capacity();
        let status = BrotliDecoderDecompress(
            data.len(),
            data.as_ptr(),
            &mut size,
            buffer.as_mut_ptr(),
        );
        if status != BROTLI_RES_SUCCESS {
            return Err(format!("failed to decompress data {status}"));
        }
        buffer.set_len(size);
        Ok(buffer)
    }
}
