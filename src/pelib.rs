use std::ffi::c_void;

use windows_sys::Win32::System::{
    Diagnostics::Debug::{IMAGE_DIRECTORY_ENTRY_BASERELOC, IMAGE_DIRECTORY_ENTRY_IMPORT},
    LibraryLoader::{GetProcAddress, LoadLibraryA},
    SystemServices::{
        IMAGE_BASE_RELOCATION, IMAGE_DOS_HEADER, IMAGE_IMPORT_DESCRIPTOR, IMAGE_NT_SIGNATURE,
    },
};

/// Function to get the size of the headers
///
/// # Arguments
///
/// * `buffer` - A slice of bytes representing the buffer.
///
/// # Returns
///
/// The size of the headers.
pub fn get_headers_size(buffer: &[u8]) -> usize {
    // Check if the first two bytes of the buffer are "MZ"
    if let Some(magic) = buffer.get(0..2) {
        let magicstring = String::from_utf8_lossy(magic);
        if magicstring == "MZ" {
            // Get the offset to the NT header
            if let Some(ntoffset) = buffer.get(60..64) {
                let offset = unsafe { std::ptr::read(ntoffset.as_ptr() as *const u32) as usize };
                // Check the bit version and return the size of the headers
                match unsafe { std::ptr::read(buffer.as_ptr().add(offset + 4 + 20) as *const u16) }
                {
                    523 | 267 => {
                        let headerssize = unsafe {
                            std::ptr::read(buffer.as_ptr().add(offset + 24 + 60) as *const u32)
                        };
                        return headerssize as usize;
                    }
                    _ => panic!("invalid bit version"),
                }
            } else {
                panic!("file size is less than 64");
            }
        } else {
            panic!("it's not a PE file");
        }
    } else {
        panic!("file size is less than 2");
    }
}

// Function to get the size of the image
/// This function returns the size of the image.
///
/// # Arguments
///
/// * `buffer` - A slice of bytes representing the buffer.
///
/// # Returns
///
/// The size of the image.
pub fn get_image_size(buffer: &[u8]) -> usize {
    // Get the magic string from the buffer
    let magic = &buffer[0..2];
    // Convert the magic string to a string
    let magicstring = std::str::from_utf8(magic).expect("invalid magic string");
    // Check if the magic string is "MZ"
    assert_eq!(magicstring, "MZ", "it's not a PE file");
    // Get the offset to the NT header
    let offset = unsafe {
        let ntoffset = &buffer[60..64];
        std::ptr::read_unaligned(ntoffset.as_ptr() as *const i32) as usize
    };
    // Get the bit version from the buffer
    let bit = unsafe {
        let bitversion = &buffer[offset + 4 + 20..offset + 4 + 20 + 2];
        std::ptr::read_unaligned(bitversion.as_ptr() as *const u16)
    };
    // Check the bit version and return the size of the image
    match bit {
        523 | 267 => {
            let index = offset + 24 + 60 - 4;
            let size = unsafe {
                let headerssize = &buffer[index..index + 4];
                std::ptr::read_unaligned(headerssize.as_ptr() as *const i32)
            };
            size as usize
        }
        _ => panic!("invalid bit version"),
    }
}

/// Function to get the DOS header
///
/// # Arguments
///
/// * `lp_image` - A pointer to the image.
///
/// # Returns
///
/// A pointer to the DOS header.
pub fn get_dos_header(lp_image: *const c_void) -> *const IMAGE_DOS_HEADER {
    lp_image as *const IMAGE_DOS_HEADER
}

/// Function to get the NT header
///
/// # Arguments
///
/// * `lp_image` - A pointer to the image.
/// * `lp_dos_header` - A pointer to the DOS header.
///
/// # Returns
///
/// A pointer to the NT header.
pub fn get_nt_header(
    lp_image: *const c_void,
    lp_dos_header: *const IMAGE_DOS_HEADER,
) -> *const c_void {
    // Calculate the address of the NT header
    let lp_nt_header = unsafe {
        (lp_image as usize + (*lp_dos_header).e_lfanew as usize)
            as *const windows_sys::Win32::System::Diagnostics::Debug::IMAGE_NT_HEADERS32
    };
    // Check if the NT header signature is valid
    if unsafe { (*lp_nt_header).Signature } != IMAGE_NT_SIGNATURE {
        return std::ptr::null_mut();
    }
    lp_nt_header as *const c_void
}

/// Returns the size of the NT header based on the target architecture.
///
/// # Returns
///
/// The size of the NT header.
fn get_nt_header_size() -> usize {
    #[cfg(target_arch = "x86")]
    return std::mem::size_of::<windows_sys::Win32::System::Diagnostics::Debug::IMAGE_NT_HEADERS32>(
    );
    #[cfg(target_arch = "x86_64")]
    return std::mem::size_of::<windows_sys::Win32::System::Diagnostics::Debug::IMAGE_NT_HEADERS64>(
    );
}

/// Returns the number of sections in the PE file based on the target architecture.
///
/// # Arguments
///
/// * `ntheader` - A pointer to the NT header of the PE file.
///
/// # Returns
///
/// The number of sections in the PE file.
fn get_number_of_sections(ntheader: *const c_void) -> u16 {
    #[cfg(target_arch = "x86_64")]
    return unsafe {
        (*(ntheader as *const windows_sys::Win32::System::Diagnostics::Debug::IMAGE_NT_HEADERS64))
            .FileHeader
            .NumberOfSections
    };
    #[cfg(target_arch = "x86")]
    return unsafe {
        (*(ntheader as *const windows_sys::Win32::System::Diagnostics::Debug::IMAGE_NT_HEADERS32))
            .FileHeader
            .NumberOfSections
    };
}

/// Writes each section of the PE file to the allocated memory in the target process.
///
/// # Arguments
///
/// * `baseptr` - A pointer to the base address of the allocated memory in the target process.
/// * `buffer` - A vector containing the bytes of the PE file to be loaded.
/// * `ntheader` - A pointer to the NT header of the PE file.
/// * `dosheader` - A pointer to the DOS header of the PE file.
pub fn write_sections(
    // A handle to the process into which the PE file will be loaded.
    // A pointer to the base address of the allocated memory in the target process.
    baseptr: *const c_void,
    // A vector containing the bytes of the PE file to be loaded.
    buffer: Vec<u8>,
    // A pointer to the NT header of the PE file.
    ntheader: *const c_void,
    // A pointer to the DOS header of the PE file.
    dosheader: *const IMAGE_DOS_HEADER,
) {
    let number_of_sections = get_number_of_sections(ntheader);
    let nt_header_size = get_nt_header_size();

    let e_lfanew = (unsafe { *dosheader }).e_lfanew as usize;
    let mut st_section_header = (baseptr as usize + e_lfanew + nt_header_size)
        as *const windows_sys::Win32::System::Diagnostics::Debug::IMAGE_SECTION_HEADER;

    for _i in 0..number_of_sections {
        // Get the section data
        let section_data = buffer
            .get(
                (unsafe { *st_section_header }).PointerToRawData as usize..((unsafe {
                    *st_section_header
                })
                .PointerToRawData
                    as usize
                    + (unsafe { *st_section_header }).SizeOfRawData as usize),
            )
            .unwrap_or_default();

        // Write the section data to the allocated memory
        unsafe {
            std::ptr::copy_nonoverlapping(
                section_data.as_ptr() as *const c_void,
                (baseptr as usize + (*st_section_header).VirtualAddress as usize) as *mut c_void,
                (*st_section_header).SizeOfRawData as usize,
            )
        };

        st_section_header = unsafe { st_section_header.add(1) };
    }
}

/// This function fixes the base relocations of the PE file in the allocated memory in the target process.
///
/// # Arguments
///
/// * `baseptr` - A pointer to the base address of the allocated memory in the target process.
/// * `ntheader` - A pointer to the NT header of the PE file.
pub fn fix_base_relocations(
    // Pointer to the base address of the allocated memory in the target process
    baseptr: *const c_void,
    // Pointer to the NT header of the PE file
    ntheader: *const c_void,
) {
    // Get the NT header
    #[cfg(target_arch = "x86_64")]
    let nt_header = unsafe {
        &(*(ntheader as *const windows_sys::Win32::System::Diagnostics::Debug::IMAGE_NT_HEADERS64))
            .OptionalHeader
    };
    #[cfg(target_arch = "x86")]
    let nt_header = unsafe {
        &(*(ntheader as *const windows_sys::Win32::System::Diagnostics::Debug::IMAGE_NT_HEADERS32))
            .OptionalHeader
    };

    // Get the base relocation directory
    let basereloc = nt_header.DataDirectory[IMAGE_DIRECTORY_ENTRY_BASERELOC as usize];
    if basereloc.Size == 0 {
        return;
    }

    // Calculate the difference between the image base and the allocated memory base
    let image_base = nt_header.ImageBase;
    let diffaddress = baseptr as usize - image_base as usize;

    // Get the pointer to the base relocation block
    let mut relocptr =
        (baseptr as usize + basereloc.VirtualAddress as usize) as *const IMAGE_BASE_RELOCATION;

    // Iterate through each block in the base relocation directory
    while unsafe { (*relocptr).SizeOfBlock } != 0 {
        // Get the number of entries in the current block
        let entries = (unsafe { (*relocptr).SizeOfBlock }
            - std::mem::size_of::<IMAGE_BASE_RELOCATION>() as u32)
            / 2;

        // Iterate through each entry in the current block
        for i in 0..entries {
            // Get the pointer to the current relocation offset
            let relocoffset_ptr = (relocptr as usize
                + std::mem::size_of::<IMAGE_BASE_RELOCATION>()
                + i as usize * 2) as *const u16;

            // Get the value of the current relocation offset
            let temp = unsafe { *relocoffset_ptr };

            // Check if the relocation type is not absolute
            if temp as u32 >> 12 as u32
                != windows_sys::Win32::System::SystemServices::IMAGE_REL_BASED_ABSOLUTE
            {
                // Calculate the final address of the relocation
                let finaladdress = baseptr as usize
                    + unsafe { (*relocptr).VirtualAddress } as usize
                    + (temp & 0x0fff) as usize;

                // Read the original value at the final address
                let ogaddress = unsafe { std::ptr::read(finaladdress as *const usize) };

                // Calculate the fixed address of the relocation
                let fixedaddress = (ogaddress + diffaddress as usize) as usize;

                // Write the fixed address to the final address
                unsafe {
                    std::ptr::write(finaladdress as *mut usize, fixedaddress);
                }
            }
        }

        // Move to the next block in the base relocation directory
        relocptr = unsafe {
            (relocptr as *const u8).add((*relocptr).SizeOfBlock as usize)
                as *const IMAGE_BASE_RELOCATION
        };
    }
}

/// Gets the import directory from the NT header of the PE file.
///
/// # Arguments
///
/// * `ntheader` - A pointer to the NT header of the PE file.
///
/// # Returns
///
/// The import directory of the PE file.
fn get_import_directory(
    ntheader: *const c_void,
) -> windows_sys::Win32::System::Diagnostics::Debug::IMAGE_DATA_DIRECTORY {
    #[cfg(target_arch = "x86_64")]
    return unsafe {
        (*(ntheader as *const windows_sys::Win32::System::Diagnostics::Debug::IMAGE_NT_HEADERS64))
            .OptionalHeader
            .DataDirectory[IMAGE_DIRECTORY_ENTRY_IMPORT as usize]
    };

    #[cfg(target_arch = "x86")]
    return unsafe {
        (*(ntheader as *const windows_sys::Win32::System::Diagnostics::Debug::IMAGE_NT_HEADERS32))
            .OptionalHeader
            .DataDirectory[IMAGE_DIRECTORY_ENTRY_IMPORT as usize]
    };
}

/// Writes the import table of the PE file to the allocated memory in the target process.
///
/// # Arguments
///
/// * `baseptr` - A pointer to the base address of the allocated memory in the target process.
/// * `ntheader` - A pointer to the NT header of the PE file.
pub fn write_import_table(
    // A pointer to the base address of the allocated memory in the target process.
    baseptr: *const c_void,
    // A pointer to the NT header of the PE file.
    ntheader: *const c_void,
) {
    // Get the import directory
    let import_dir = get_import_directory(ntheader);

    // If the import directory is empty, return
    if import_dir.Size == 0 {
        return;
    }

    // Get the pointer to the first thunk
    let mut ogfirstthunkptr = baseptr as usize + import_dir.VirtualAddress as usize;

    // Loop through each import descriptor
    while unsafe { (*(ogfirstthunkptr as *const IMAGE_IMPORT_DESCRIPTOR)).Name } != 0
        && unsafe { (*(ogfirstthunkptr as *const IMAGE_IMPORT_DESCRIPTOR)).FirstThunk } != 0
    {
        // Get the import descriptor
        let mut import = unsafe { std::mem::zeroed::<IMAGE_IMPORT_DESCRIPTOR>() };
        //fill_structure_from_memory(&mut import, ogfirstthunkptr as *const c_void);
        unsafe {
            std::ptr::copy_nonoverlapping(
                ogfirstthunkptr as *const u8,
                &mut import as *mut IMAGE_IMPORT_DESCRIPTOR as *mut u8,
                std::mem::size_of::<IMAGE_IMPORT_DESCRIPTOR>(),
            );
        }
        // Get the name of the DLL
        let dllname = crate::utils::read_string_from_memory(
            (baseptr as usize + import.Name as usize) as *const u8,
        );

        // Load the DLL
        let dllhandle = unsafe { LoadLibraryA(dllname.as_bytes().as_ptr() as *const u8) };

        // Get the pointer to the first thunk for this import descriptor
        let mut thunkptr = unsafe {
            baseptr as usize
                + (import.Anonymous.OriginalFirstThunk as usize
                    | import.Anonymous.Characteristics as usize)
        };

        let mut i = 0;

        // Loop through each thunk for this import descriptor
        // and replace the function address with the address of the function in the DLL
        while unsafe { *(thunkptr as *const usize) } != 0 {
            // Get the thunk data
            let mut thunkdata: [u8; std::mem::size_of::<usize>()] =
                unsafe { std::mem::zeroed::<[u8; std::mem::size_of::<usize>()]>() };
            unsafe {
                std::ptr::copy_nonoverlapping(
                    thunkptr as *const u8,
                    &mut thunkdata as *mut u8,
                    std::mem::size_of::<usize>(),
                );
            }
            // Get the offset of the function name
            let offset = usize::from_ne_bytes(thunkdata);

            // Get the function name
            let funcname = crate::utils::read_string_from_memory(
                (baseptr as usize + offset as usize + 2) as *const u8,
            );

            // If the function name is not empty, replace the function address with the address of the function in the DLL
            if !funcname.is_empty() {
                let funcaddress = unsafe {
                    GetProcAddress(dllhandle, funcname.as_bytes().as_ptr() as *const u8).unwrap()
                };
                let funcaddress_ptr = (baseptr as usize
                    + import.FirstThunk as usize
                    + i * std::mem::size_of::<usize>())
                    as *mut usize;
                unsafe { std::ptr::write(funcaddress_ptr, funcaddress as usize) };
            }
            i += 1;
            // Move to the next thunk
            thunkptr += std::mem::size_of::<usize>();
        }
        ogfirstthunkptr += std::mem::size_of::<IMAGE_IMPORT_DESCRIPTOR>();
    }
}