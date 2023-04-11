# rspe

Simple Native Rust Reflective PE loader library

## Features

This project can execute RunPE into memory using the following methods:

- Native RunPE (C/C++/RUST...)
    - [x] 64-bit
    - [x] 32-bit 
- .NET RunPE (C#/VB/CLR...)
    - [ ] 64-bit .NET RunPE into Memory (maybe view [clroxide lib](https://github.com/yamakadi/clroxide))
    - [ ] 32-bit .NET RunPE into Memory (maybe view [clroxide lib](https://github.com/yamakadi/clroxide))

## Use

```rust
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
```

## Credits / References

Special thanks to the following individuals and projects for their contributions to this project:

- [memN0ps](https://github.com/memN0ps) for providing useful winapi rust code for learning
- [trickster0](https://github.com/trickster0) for providing many OffensiveRust code for learning

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.