use std::path::{Path, PathBuf};

use clap::Parser;

use crate::cli::Cli;
use crate::utils::{Role, buffered_concat, open_all_row_sources, stream_concat};
use crate::{ProcessError, TextureFormat};

#[derive(Parser, Clone)]
pub struct RgbaConcat {
    #[arg(short, long)]
    pub red: PathBuf,
    #[arg(short, long)]
    pub green: PathBuf,
    #[arg(short, long)]
    pub blue: PathBuf,
    #[arg(short, long)]
    pub alpha: Option<PathBuf>,
    #[arg(short, long)]
    pub output: PathBuf,
    #[arg(long, short, default_value_t = TextureFormat::default())]
    pub format: TextureFormat,
}

pub fn rgba_concat(global: Cli, args: RgbaConcat) -> Result<(), ProcessError> {
    if args.output.exists() && !global.overwrite {
        println!(
            "Output file `{}` already exists!",
            args.output.to_string_lossy()
        );
        return Ok(());
    }

    let mut roles: Vec<(Role, &Path)> = vec![
        (Role::Red, args.red.as_path()),
        (Role::Green, args.green.as_path()),
        (Role::Blue, args.blue.as_path()),
    ];
    if let Some(alpha) = &args.alpha {
        roles.push((Role::Alpha, alpha.as_path()));
    }
    let out_channels = if args.alpha.is_some() { 4 } else { 3 };

    match open_all_row_sources(&roles)? {
        Some(sources) => stream_concat(
            sources,
            out_channels,
            &args.output,
            args.format,
            global.display_format,
        ),
        None => buffered_concat(
            &roles,
            out_channels,
            &args.output,
            args.format,
            global.no_memory_limit,
            global.display_format,
        ),
    }
}
