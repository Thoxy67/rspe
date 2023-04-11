// Set the windows subsystem to "windows" if the target OS is Windows and debug assertions are not enabled
#![cfg_attr(
    all(target_os = "windows", not(debug_assertions),),
    windows_subsystem = "windows"
)]

use rspe::{reflective_loader, utils::check_dotnet};

// Main function
fn main() -> Result<(), String> {
    // Read the file to load into a buffer
    #[cfg(target_arch = "x86_64")]
    let data = include_bytes!(r#".\putty_x64.exe"#).to_vec();
    #[cfg(target_arch = "x86")]
    let data = include_bytes!(r#".\putty_x86.exe"#).to_vec();

    // Load the file based on the target architecture
    // Check if the file is a .NET assembly
    if !check_dotnet(data.clone()) {
        // If it is not, use the reflective loader to load the file
        unsafe {
            reflective_loader(data.clone());

            // Using Threads (useful to bind 2nd exe to execute at the same time):
            // Currently not in use, but can be used to load the pe file in a separate thread
            // let handle = std::thread::spawn(move || {
            //     pe::loader::reflective_loader(data.clone());
            // });
            // let _ = handle.join();
        };
    } else {
        panic!("This is a .NET PE file. Only native PE image are supported! Please provide a native PE image.")
    }

    Ok(())
}
