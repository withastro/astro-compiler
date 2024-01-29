use core::slice;
use std::mem::MaybeUninit;

use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::SourceType;
use serde_json;
use web_sys::console;

// A function that takes a string of TypeScript source code
// and prints its AST in JSON format.
pub fn print_ast(source_text: &String) -> String {
    // the source text is always typescript in Astro
    const FILE_NAME_OF_TYPE: &str = "template.ts";
    let source_type = SourceType::from_path(FILE_NAME_OF_TYPE).unwrap();

    let allocator = Allocator::default();
    let ret = Parser::new(&allocator, source_text, source_type).parse();

    if ret.errors.is_empty() {
        let json_string = format!("{}", serde_json::to_string_pretty(&ret.program).unwrap());
        console::log_1(&json_string.clone().into());
        return json_string.into();
    } else {
        console::log_1(&"A TypeScript error occured in your Astro component".into());
        // let's not handle errors for now
        return "{\"hey\": \"there\"}".to_string().into();
        // for error in ret.errors {
        //     let error = error.with_source_code(source_text.clone());
        //     let error = format!("{error:?}");
        //     // console::log_1(&error.into());
        //     return error;
        // }
    }
}

/// WebAssembly export that accepts a string (linear memory offset, byteCount)
/// and returns a pointer/size pair packed into a u64.
///
/// Note: The return value is leaked to the caller, so it must call
/// [`deallocate`] when finished.
/// Note: This uses a u64 instead of two result values for compatibility with
/// WebAssembly 1.0.
#[cfg_attr(all(target_arch = "wasm32"), export_name = "print_ast")]
#[no_mangle]
pub unsafe extern "C" fn _print_ast(ptr: u32, len: u32) -> u64 {
    let source_text = &ptr_to_string(ptr, len);
    let g = print_ast(source_text);
    let (ptr, len) = string_to_ptr(&g);
    // Note: This changes ownership of the pointer to the external caller. If
    // we didn't call forget, the caller would read back a corrupt value. Since
    // we call forget, the caller must deallocate externally to prevent leaks.
    std::mem::forget(g);
    return ((ptr as u64) << 32) | len as u64;
}

/// Returns a string from WebAssembly compatible numeric types representing
/// its pointer and length.
unsafe fn ptr_to_string(ptr: u32, len: u32) -> String {
    let slice = slice::from_raw_parts_mut(ptr as *mut u8, len as usize);
    let utf8 = std::str::from_utf8_unchecked_mut(slice);
    return String::from(utf8);
}

/// Returns a pointer and size pair for the given string in a way compatible
/// with WebAssembly numeric types.
///
/// Note: This doesn't change the ownership of the String. To intentionally
/// leak it, use [`std::mem::forget`] on the input after calling this.
unsafe fn string_to_ptr(s: &String) -> (u32, u32) {
    return (s.as_ptr() as u32, s.len() as u32);
}

/// Set the global allocator to the WebAssembly optimized one.
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

/// WebAssembly export that allocates a pointer (linear memory offset) that can
/// be used for a string.
///
/// This is an ownership transfer, which means the caller must call
/// [`deallocate`] when finished.
#[cfg_attr(all(target_arch = "wasm32"), export_name = "allocate")]
#[no_mangle]
pub extern "C" fn _allocate(size: u32) -> *mut u8 {
    allocate(size as usize)
}

/// Allocates size bytes and leaks the pointer where they start.
fn allocate(size: usize) -> *mut u8 {
    // Allocate the amount of bytes needed.
    let vec: Vec<MaybeUninit<u8>> = Vec::with_capacity(size);

    // into_raw leaks the memory to the caller.
    Box::into_raw(vec.into_boxed_slice()) as *mut u8
}

/// WebAssembly export that deallocates a pointer of the given size (linear
/// memory offset, byteCount) allocated by [`allocate`].
#[cfg_attr(all(target_arch = "wasm32"), export_name = "deallocate")]
#[no_mangle]
pub unsafe extern "C" fn _deallocate(ptr: u32, size: u32) {
    deallocate(ptr as *mut u8, size as usize);
}

/// Retakes the pointer which allows its memory to be freed.
unsafe fn deallocate(ptr: *mut u8, size: usize) {
    let _ = Vec::from_raw_parts(ptr, 0, size);
}
