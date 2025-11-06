//! sapling: A syntactic patching library with char-level granularity.
//!
//! Command-line interface for applying patches from JSON.
//!
//! `sapling` provides a robust way to apply patches to source files using rope data structures
//! for efficient editing and tree-sitter for syntactic awareness. Unlike traditional line-based
//! patch formats, sapling operates at character granularity and can compose multiple patches
//! with automatic offset tracking.
//!
//! Reads a JSON array of patches from a file or stdin and applies them to their target files.
//! Modified files are written back to disk unless `--dry-run` is specified.
#![allow(clippy::multiple_crate_versions)]

/// Command-line interface for applying patches from JSON.
#[cfg(feature = "cli")]
pub mod inner {
    use facet::Facet;
    use sapling::{Patch, PatchSet};
    use std::fs;
    use std::io::{self, Read};

    #[derive(Facet)]
    struct Args {
        /// Path to JSON file containing patches (reads from stdin if not provided)
        #[facet(positional, default)]
        patch_file: Option<String>,

        /// Preview changes without writing to disk
        #[facet(named, short = 'n')]
        dry_run: bool,

        /// Show verbose output
        #[facet(named, short = 'v')]
        verbose: bool,

        /// Show this help message
        #[facet(named, short = 'h')]
        help: bool,
    }

    fn print_usage() {
        println!("Usage: sapling [OPTIONS] [PATCH_FILE]");
        println!();
        println!("Apply syntactic patches to source files with char-level granularity.");
        println!();
        println!("Arguments:");
        println!("  [PATCH_FILE]  Path to JSON file containing patches (reads from stdin if not provided)");
        println!();
        println!("Options:");
        println!("  -n, --dry-run  Preview changes without writing to disk");
        println!("  -v, --verbose  Show verbose output");
        println!("  -h, --help     Show this help message");
    }

    #[cfg(feature = "cli")]
    /// Entry point for the `sapling` command-line interface.
    ///
    /// Reads JSON patches from a file or stdin and applies them to their target files.
    ///
    /// # Errors
    ///
    /// Returns an [`io::Error`] if:
    /// - command-line argument parsing fails,
    /// - the input file cannot be read,
    /// - patch JSON is malformed,
    /// - or writing the modified files fails.
    ///
    /// The process will also exit with a non-zero status if patch application fails.
    pub fn main() -> io::Result<()> {
        let args: Args = facet_args::from_std_args()
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, format!("{e}")))?;

        if args.help {
            print_usage();
            std::process::exit(0);
        }

        // Read input from file or stdin
        let input = if let Some(path) = args.patch_file {
            fs::read_to_string(&path)?
        } else {
            let mut buf = String::new();
            io::stdin().read_to_string(&mut buf)?;
            buf
        };

        // Parse patches from JSON using facet
        let patches: Vec<Patch> = facet_json::from_str(&input)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("{e:?}")))?;

        if args.verbose {
            eprintln!("Loaded {} patch(es)", patches.len());
        }

        let mut set = PatchSet::new();
        for patch in patches {
            set.add(patch);
        }

        // Apply patches
        match set.apply_to_files() {
            Ok(results) => {
                for (file, content) in results {
                    if args.dry_run {
                        eprintln!("Would patch: {file}");
                        if args.verbose {
                            println!("=== {file} ===\n{content}");
                        }
                    } else {
                        fs::write(&file, content)?;
                        eprintln!("Patched: {file}");
                    }
                }

                if args.dry_run && !args.verbose {
                    eprintln!("Dry run complete. Use -v to see changes.");
                }
            }
            Err(e) => {
                eprintln!("Error: {e}");
                std::process::exit(1);
            }
        }

        Ok(())
    }
}

/// Hint replacement CLI for when the cli module is used without building the cli feature.
#[cfg(not(feature = "cli"))]
pub mod inner {
    /// Provide a hint to the user that they did not build this crate with the cli feature.
    #[cfg(not(feature = "cli"))]
    pub fn main() {
        eprintln!("Please build with the cli feature to run the CLI");
        std::process::exit(1);
    }
}

pub use inner::main;
