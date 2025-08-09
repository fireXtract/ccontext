//! # ccontext - Flexible Windows Path Transformation Tool
//!
//! This tool converts Windows file paths into various other formats (like WSL, Cygwin, Docker)
//! by applying a series of transformations. It's designed to be called from the
//! Windows Explorer context menu.
//!
//! The logic is a Rust implementation of the concepts in the original `CopyContext.ps1`
//! script, with added robustness for UNC paths and more flexible mapping options.

// Use this attribute to hide the console window for the final release.
// Comment it out during development to see `println!` and `eprintln!` output.
#![windows_subsystem = "windows"]

use clap::{Arg, ArgAction, Command};
use clipboard_win::{set_clipboard_string};

/// # Main Application Logic
///
/// This function orchestrates the entire process:
/// 1. Parses command-line arguments using `clap`.
/// 2. Determines which transformation preset is being used (e.g., WSL, Cygwin).
/// 3. Calls the core `transform_path` function to perform the conversion.
/// 4. Either prints the result to the console (`--dry-run`) or copies it to the clipboard.
fn main() {
    // Define the command-line interface for the application.
    let matches = Command::new("ccontext")
        .version("1.0.0")
        .author("Gemini")
        .about("A flexible script to convert Windows file paths into various other formats.")
        .arg(
            Arg::new("path")
                .help("The full Windows path to a file or folder (e.g., from %1).")
                .required(true)
                .index(1),
        )
        .arg(
            Arg::new("prefix")
                .long("prefix")
                .help("A string to prepend to the beginning of the final, transformed path.")
                .num_args(1)
                .default_value(""),
        )
        .arg(
            Arg::new("strip_drive")
                .long("strip-drive")
                .help("Removes the drive letter (e.g., 'C:') or UNC root from the path.")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("map_drive_to_prefix")
                .long("map-drive-to-prefix")
                .help("Converts a drive letter like 'C:' to a prefix 'c/'.")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("strip_leading_path")
                .long("strip-leading-path")
                .help("A string to remove from the beginning of the path body.")
                .num_args(1),
        )
        .arg(
            Arg::new("convert_to_forward_slashes")
                .long("convert-to-forward-slashes")
                .help("Converts all backslashes '\\' to forward slashes '/'.")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("dry_run")
                .long("dry-run")
                .help("Prints the final path to the console instead of copying to the clipboard.")
                .action(ArgAction::SetTrue),
        )
        .get_matches();

    // --- Argument Extraction ---
    let input_path = matches.get_one::<String>("path").unwrap();

    // The .reg file will call the executable with the correct combination of flags
    // to achieve the desired transformation.
    let options = TransformOptions {
        prefix: matches.get_one::<String>("prefix").unwrap().to_string(),
        strip_drive: matches.get_flag("strip_drive"),
        map_drive_to_prefix: matches.get_flag("map_drive_to_prefix"),
        strip_leading_path: matches.get_one::<String>("strip_leading_path").map(|s| s.to_string()),
        convert_to_forward_slashes: matches.get_flag("convert_to_forward_slashes"),
    };

    // --- Transformation ---
    let final_path = transform_path(input_path, &options);

    // --- Output ---
    if matches.get_flag("dry_run") {
        println!("{}", final_path);
    } else {
        match set_clipboard_string(&final_path) {
            Ok(_) => (), // Success, do nothing.
            Err(e) => eprintln!("Error: Failed to set clipboard. {}", e),
        }
    }
}

/// # Transformation Options
///
/// A struct to hold all the configurable options for the path transformation.
/// This makes passing arguments to the core logic function cleaner.
#[derive(Debug, Default)]
struct TransformOptions {
    prefix: String,
    strip_drive: bool,
    map_drive_to_prefix: bool,
    strip_leading_path: Option<String>,
    convert_to_forward_slashes: bool,
}

/// # Path Components
///
/// Represents a Windows path split into its root (drive letter or UNC share)
/// and the remaining body of the path.
#[derive(Debug, PartialEq)]
struct PathComponents<'a> {
    /// The root part of the path, e.g., `C:` or `\\server\share`.
    root: Option<&'a str>,
    /// The rest of the path after the root, e.g., `\Users\Test\file.txt`.
    body: &'a str,
}

/// # Splits a Windows path into its root and body.
///
/// This function correctly handles both standard drive paths (C:\...) and
/// UNC paths (\\server\share\...).
///
/// ## Examples
/// - `C:\Users\Test` -> `root: Some("C:"), body: "\Users\Test"`
/// - `\\192.168.0.123\backup\zest` -> `root: Some("\\192.168.0.123\backup"), body: "\zest"`
/// - `relative\path` -> `root: None, body: "relative\path"`
fn split_path(path_str: &str) -> PathComponents<'_> {
    // Normalize path by removing quotes, which are often added by Windows' %1.
    let mut clean_path = path_str.trim();
    if clean_path.starts_with('"') && clean_path.ends_with('"') {
        clean_path = &clean_path[1..clean_path.len() - 1];
    }

    // Handle UNC paths: \\server\share\rest\of\path
    if clean_path.starts_with(r"\\") {
        let mut parts = clean_path.trim_start_matches(r"\\").splitn(3, r"\");
        let server = parts.next().unwrap_or("");
        let share = parts.next().unwrap_or("");
        //let rest = parts.next().unwrap_or("");

        if !server.is_empty() && !share.is_empty() {
            let root_end = r"\\".len() + server.len() + r"\".len() + share.len();
            // The body needs to include the leading slash if it exists
            let body = if clean_path.len() > root_end { &clean_path[root_end..] } else { "" };
            return PathComponents {
                root: Some(&clean_path[..root_end]),
                body,
            };
        }
    }

    // Handle drive letter paths: C:\rest\of\path
    if let Some(drive_colon) = clean_path.get(0..2) {
        if drive_colon.ends_with(':') && drive_colon.chars().next().unwrap().is_ascii_alphabetic() {
            let body = clean_path.get(2..).unwrap_or("");
            return PathComponents {
                root: Some(drive_colon),
                body,
            };
        }
    }

    // If neither, treat the whole thing as the body.
    PathComponents {
        root: None,
        body: clean_path,
    }
}


/// # Core Path Transformation Logic
///
/// This is the heart of the tool. It takes a path and a set of options
/// and performs the conversion according to the rules defined in the
/// original PowerShell script.
///
/// The order of operations is critical and is preserved here:
/// 1. The path root (drive/UNC) is handled.
/// 2. Slashes are converted.
/// 3. A leading path component is stripped.
/// 4. The final prefix is prepended.
fn transform_path(path_str: &str, options: &TransformOptions) -> String {
    let components = split_path(path_str);
    let mut processed_body = components.body.to_string();
    let mut root_prefix = String::new();

    // --- 1. Handle the path root (Drive or UNC) ---
    if let Some(root) = components.root {
        if options.map_drive_to_prefix {
            // This option is mainly for drive letters. We take the letter,
            // lowercase it, and make it a prefix like "c/".
            if root.ends_with(':') {
                let drive_letter = root.get(..1).unwrap_or("").to_lowercase();
                root_prefix = format!("{}/", drive_letter);
            }
        } else if options.strip_drive {
            // Do nothing; the root is discarded.
        } else {
            // Default behavior: keep the root as-is.
            root_prefix = root.to_string();
        }
    }

    // --- 2. Convert slashes if requested ---
    // This is done before stripping the leading path so the strip pattern
    // can use forward slashes.
    if options.convert_to_forward_slashes {
        processed_body = processed_body.replace('\\', "/");
    }

    // --- 3. Strip a leading part of the path if specified ---
    if let Some(strip_pattern) = &options.strip_leading_path {
        let mut pattern = strip_pattern.clone();
        // Ensure the pattern uses the same slash type as the processed body.
        if options.convert_to_forward_slashes {
            pattern = pattern.replace('\\', "/");
        } else {
            pattern = pattern.replace('/', "\\");
        }

        // Normalize the body and pattern for a more robust match by removing
        // any leading/trailing slashes from the comparison strings.
        let normalized_body = processed_body.trim_start_matches(&['/', '\\']);
        let normalized_pattern = pattern.trim_end_matches(&['/', '\\']);

        if normalized_body.to_lowercase().starts_with(&normalized_pattern.to_lowercase()) {
            // Find the actual start of the rest of the path after the pattern
            let strip_len = pattern.len();
            processed_body = processed_body[strip_len..].to_string();
        }
    }

    // --- 4. Assemble the final path ---
    let mut final_path = format!("{}{}{}", options.prefix, root_prefix, processed_body);

    // --- Final Cleanup ---
    // If we converted to forward slashes, clean up any double slashes that might
    // have been created by joining prefixes (e.g., "/mnt/" + "/c/path").
    // We are careful not to remove the double slash in protocol prefixes like `file://`
    if options.convert_to_forward_slashes {
        let mut result = String::with_capacity(final_path.len());
        let mut last_char = ' ';
        for (i, current_char) in final_path.chars().enumerate() {
            // Allow a leading `//` for UNC-style paths but not elsewhere.
            if current_char == '/' && last_char == '/' && i > 1 {
                // Skip this character
            } else {
                result.push(current_char);
            }
            last_char = current_char;
        }
        final_path = result;
    }

    final_path
}


// #############################################################################
// # UNIT TESTS
// #############################################################################
// These tests verify that the transformation logic is correct for all
// scenarios described by the user and in the original script examples.
// Run tests with `cargo test`.
#[cfg(test)]
mod tests {
    use super::*;

    // Helper function to create default options for testing.
    fn opts(
        prefix: &str,
        strip_drive: bool,
        map_drive: bool,
        strip_leading: Option<&str>,
        convert_slashes: bool,
    ) -> TransformOptions {
        TransformOptions {
            prefix: prefix.to_string(),
            strip_drive,
            map_drive_to_prefix: map_drive,
            strip_leading_path: strip_leading.map(String::from),
            convert_to_forward_slashes: convert_slashes,
        }
    }

    #[test]
    fn test_split_path_components() {
        assert_eq!(split_path(r#""C:\Users\Test""#), PathComponents { root: Some("C:"), body: r"\Users\Test" });
        assert_eq!(split_path(r"L:\zest\t\another.txt"), PathComponents { root: Some("L:"), body: r"\zest\t\another.txt" });
        assert_eq!(split_path(r"\\10.0.0.5\backup\zest\t\another.txt"), PathComponents { root: Some(r"\\10.0.0.5\backup"), body: r"\zest\t\another.txt" });
        assert_eq!(split_path(r"\\server\share"), PathComponents { root: Some(r"\\server\share"), body: "" });
        assert_eq!(split_path(r"just_a_file.txt"), PathComponents { root: None, body: "just_a_file.txt" });
    }

    #[test]
    fn test_scenario_wsl() {
        // Desired: C:\Users\Me\file.txt -> /mnt/c/Users/Me/file.txt
        let options = opts("/mnt/", false, true, None, true);
        let result = transform_path(r"C:\Users\Me\file.txt", &options);
        assert_eq!(result, "/mnt/c/Users/Me/file.txt");
    }

    #[test]
    fn test_scenario_cygwin() {
        // Desired: C:\Users\Me\file.txt -> /cygdrive/c/Users/Me/file.txt
        let options = opts("/cygdrive/", false, true, None, true);
        let result = transform_path(r"C:\Users\Me\file.txt", &options);
        assert_eq!(result, "/cygdrive/c/Users/Me/file.txt");
    }

    #[test]
    fn test_scenario_git_bash() {
        // Desired: C:\Users\Me\file.txt -> /c/Users/Me/file.txt
        let options = opts("/", false, true, None, true);
        let result = transform_path(r"C:\Users\Me\file.txt", &options);
        assert_eq!(result, "/c/Users/Me/file.txt");
    }

    #[test]
    fn test_scenario_remote_server_l_drive() {
        // Desired: L:\zest\t\another.txt -> /media/zest/t/another.txt
        let options = opts("/media/", true, false, None, true);
        let result = transform_path(r"L:\zest\t\another.txt", &options);
        assert_eq!(result, "/media/zest/t/another.txt");
    }

    #[test]
    fn test_scenario_remote_server_unc() {
        // Desired: \\10.0.0.5\backup\zest\t\another.txt -> /media/zest/t/another.txt
        // This requires stripping the UNC root.
        let options = opts("/media/", true, false, None, true);
        let result = transform_path(r"\\10.0.0.5\backup\zest\t\another.txt", &options);
        assert_eq!(result, "/media/zest/t/another.txt");
    }

    #[test]
    fn test_scenario_docker_l_drive() {
        // Desired: L:\zest\t\another.txt -> /data/another.txt
        let options = opts("/data/", true, false, Some("zest/t/"), true);
        let result = transform_path(r"L:\zest\t\another.txt", &options);
        assert_eq!(result, "/data/another.txt");
    }

    #[test]
    fn test_scenario_docker_unc() {
        // Desired: \\10.0.0.5\backup\zest\t\another.txt -> /data/another.txt
        let options = opts("/data/", true, false, Some("zest/t/"), true);
        let result = transform_path(r"\\10.0.0.5\backup\zest\t\another.txt", &options);
        assert_eq!(result, "/data/another.txt");
    }

    #[test]
    fn test_scenario_dev_mount_j_drive() {
        // Desired: J:\my-project -> /home/who/dev/my-project
        // The .reg file will call it like this: prefix=/home/who/, strip_drive=true, strip_leading_path=dev
        let options = opts("/home/who/dev", true, false, None, true);
        let result = transform_path(r"J:\my-project", &options);
        assert_eq!(result, "/home/who/dev/my-project");
    }

    #[test]
    fn test_scenario_dev_mount_j_drive_root() {
        // Desired: J:\ -> /home/who/
        let options = opts("/home/who/dev", true, false, None, true);
        let result = transform_path(r"J:\", &options);
        assert_eq!(result, "/home/who/dev/");
    }

    #[test]
    fn test_just_convert_slashes() {
        // Desired: C:\Users\Me -> C:/Users/Me
        let options = opts("", false, false, None, true);
        let result = transform_path(r"C:\Users\Me", &options);
        assert_eq!(result, "C:/Users/Me");
    }

    #[test]
    fn test_no_op_copy() {
        // Desired: C:\Users\Me -> C:\Users\Me
        let options = opts("", false, false, None, false);
        let result = transform_path(r"C:\Users\Me", &options);
        assert_eq!(result, r"C:\Users\Me");
    }
}
