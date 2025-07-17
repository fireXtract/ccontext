// Many thanks to Gemini 2.5 Pro
#![windows_subsystem = "windows"] 
// Comment above out when debugging, it prevents a window from flashing when the
// binary is invoked via context menu but also disables printing to the terminal.

use clipboard_win::set_clipboard_string;
use std::env;

/// A simple utility to transform a Windows file path from a mapped drive
/// to its corresponding Linux/Docker path and copy it to the clipboard.
fn main() {
    // Collect command-line arguments.
    // args[0] is the program path.
    // args[1] should be the full file path from the context menu ("%1").
    // args[2] may be the remote path of your network mapped drive ("/media/").
    // args[3] may be the path relative to args[2] that you'd like to strip before prefixing args[4] ("a/iso").
    // args[4] may be the path that the prior two are mounted to inside the container ("/data")
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        // eprintln!("Usage: ccontext.exe <file_path> ['/media/'] ['a/iso' '/data']");
        return;
    }

    let input_path = args.get(1).map(String::as_str).unwrap_or_default();
    let remote_server_prefix = args.get(2).map(String::as_str).unwrap_or_default();
    let docker_mount_relative_to_remote = args.get(3).map(String::as_str).unwrap_or_default();
    let inside_docker_prefix = args.get(4).map(String::as_str).unwrap_or_default();

    let mode = match (
        remote_server_prefix.is_empty(),
        docker_mount_relative_to_remote.is_empty(),
        inside_docker_prefix.is_empty(),
    ) {
        (false, false, false) => Mode::Docker,
        (false, _, _) => Mode::Remote,
        _ => Mode::Windows,
    };

    /// WINDOWS_DRIVE_PREFIX_LEN Doesn't need to be changed if your drive is mounted elsewhere;
    /// `D:\` through `Z:\` are the same length. It seems the complier turns this into a number
    /// for us. (I couldn't see it in strings)
    const WINDOWS_DRIVE_PREFIX_LEN: usize = "L:\\".len();

    // Extract the part of the path after the drive prefix.
    // e.g., for "L:\Users\Me\file.txt", this will be "Users\Me\file.txt".
    let relative_path = &input_path[WINDOWS_DRIVE_PREFIX_LEN..];

    // Transform the path based on the selected mode.
    // We replace backslashes with forward slashes for the Linux-based paths.
    let final_path = match mode {
        Mode::Windows => {
            // For the windows path, we just use the original input.
            input_path.to_string()
        }
        Mode::Remote => {
            // For the remote path, combine the server prefix with the relative path.
            format!(
                "{}{}",
                remote_server_prefix,
                relative_path.replace('\\', "/")
            )
        }
        Mode::Docker => {
            // For the docker path, combine the container prefix with the relative path.
            let relative_path = relative_path.replace('\\', "/");
            let relative_path = &relative_path[docker_mount_relative_to_remote.len()..];
            format!("{}{}", inside_docker_prefix, relative_path)
        }
    };

    match set_clipboard_string(&final_path) {
        Ok(_) => {
            // println!("Successfully copied to clipboard:\n{}", final_path);
        }
        Err(_e) => {
            // eprintln!("Error: Could not copy to clipboard. {}", _e);
        }
    }
}

enum Mode {
    Windows,
    Remote,
    Docker,
}
