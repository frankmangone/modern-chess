//! The engine, built for WebAssembly with nothing to call and nothing to be called by but the
//! host's two entry points. It imports no function: no clock, no files, no network, no random
//! numbers. All it can do is turn text into text, so the most a stranger's rules can cost is
//! the host's fuel and memory budget, and a fault is a trap the host reads, not a crash.
//!
//! Protocol: the host calls `alloc(len)`, writes the request's JSON there, calls
//! `handle(ptr, len)` and reads the response at the address in the high 32 bits of the answer,
//! its length being the low 32. Every call is a fresh instance, so nothing is ever freed.

#![cfg(target_arch = "wasm32")]

use std::slice;

/// Room for the host to write a request into.
#[no_mangle]
pub extern "C" fn alloc(len: u32) -> u32 {
    let mut buffer = Vec::<u8>::with_capacity(len as usize);
    let address = buffer.as_mut_ptr() as u32;
    std::mem::forget(buffer);
    address
}

/// Answers the request at `ptr` (`len` bytes of JSON): `(address << 32) | length` of the response.
#[no_mangle]
pub extern "C" fn handle(ptr: u32, len: u32) -> u64 {
    // SAFETY: the host wrote `len` bytes at `ptr`, which `alloc` handed out, and nothing else
    // touches this memory while the call runs.
    let request = unsafe { slice::from_raw_parts(ptr as *const u8, len as usize) };
    let response = match std::str::from_utf8(request) {
        Ok(text) => modern_chess_sandbox_protocol::handle(text),
        Err(_) => r#"{"answer":"failed","message":"not text"}"#.to_string(),
    };
    let bytes = response.into_bytes().into_boxed_slice();
    let (address, length) = (bytes.as_ptr() as u32, bytes.len() as u32);
    std::mem::forget(bytes);
    (u64::from(address) << 32) | u64::from(length)
}
