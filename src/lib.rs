pub mod pelib;
pub mod test;
pub mod utils;

use pelib::{
    fix_base_relocations, get_dos_header, get_headers_size, get_image_size, get_nt_header,
    write_import_table, write_sections,
};
use utils::detect_platform;
use windows_sys::Win32::System::Memory::{VirtualAlloc, MEM_COMMIT, PAGE_EXECUTE_READWRITE};

use std::ffi::c_void;

/// Compares the platform of the imported Portable Executable (PE) file with the platform of the compiled binary.
/// Panic if not same platforms
///
/// # Arguments
///
/// * `data` - A vector containing the bytes of the PE file to be loaded.
fn is_platforms_same(data: Vec<u8>) {
    let platform = detect_platform(&data).unwrap();

    let target_arch = if cfg!(target_arch = "x86_64") { 64 } else { 32 };

    if platform != target_arch {
        println!(
            "Compiled platform : {}bit\tNeeded platform : {}bit",
            target_arch, platform
        );
        panic!("The platform not the same as the imported pe.")
    }
}


/// Loads a Portable Executable (PE) file into memory using reflective loading.
///
/// # Arguments
///
/// * `buffer` - A vector containing the bytes of the PE file to be loaded.
///
/// # Safety
///
/// This function is unsafe because it directly interacts with the Windows API and modifies memory
/// in the target process.
pub unsafe fn reflective_loader(buffer: Vec<u8>) {
    is_platforms_same(buffer.clone());

    // Get the size of the headers and the image
    let headerssize = get_headers_size(&buffer);
    let imagesize = get_image_size(&buffer);

    // Allocate memory for the image
    let baseptr = VirtualAlloc(
        std::ptr::null_mut(), // lpAddress: A pointer to the starting address of the region to allocate.
        imagesize,            // dwSize: The size of the region, in bytes.
        MEM_COMMIT,           // flAllocationType: The type of memory allocation.
        PAGE_EXECUTE_READWRITE, // flProtect: The memory protection for the region of pages to be allocated.
    );

    // Write the headers to the allocated memory
    std::ptr::copy_nonoverlapping(buffer.as_ptr() as *const c_void, baseptr, headerssize);

    // Get the DOS header
    let dosheader = get_dos_header(buffer.as_ptr() as *const c_void);

    // Get the NT header IMAGE_NT_HEADERS64|IMAGE_NT_HEADERS32
    let ntheader = get_nt_header(buffer.as_ptr() as *const c_void, dosheader);

    // Write each section to the allocated memory
    write_sections(
        baseptr,        // The base address of the image.
        buffer.clone(), // The buffer containing the image.
        ntheader,       // The NT header of the image.
        dosheader,      // The DOS header of the image.
    );

    // Write the import table to the allocated memory
    write_import_table(baseptr, ntheader);

    // Fix the base relocations
    fix_base_relocations(baseptr, ntheader);

    #[cfg(target_arch = "x86_64")]
    let entrypoint = (baseptr as usize
        + (*(ntheader as *const windows_sys::Win32::System::Diagnostics::Debug::IMAGE_NT_HEADERS32))
            .OptionalHeader
            .AddressOfEntryPoint as usize) as *const c_void;
    #[cfg(target_arch = "x86")]
    let entrypoint = (baseptr as usize
        + (*(ntheader as *const windows_sys::Win32::System::Diagnostics::Debug::IMAGE_NT_HEADERS32))
            .OptionalHeader
            .AddressOfEntryPoint as usize) as *const c_void;

    // Create a new thread to execute the image
    execute_image(entrypoint);

    // Free the allocated memory of baseptr
    let _ = baseptr;
}

/// Executes the image by calling its entry point and waiting for the thread to finish executing.
///
/// # Arguments
///
/// * `entrypoint` - A pointer to the PE file entrypoint.
///
/// # Safety
///
/// This function is unsafe because it directly interacts with the Windows API and modifies memory
/// in the target process.
unsafe fn execute_image(entrypoint: *const c_void) {
    // Call the entry point of the image
    let func: extern "C" fn() -> u32 = std::mem::transmute(entrypoint);
    func();
}
