mod cli;
pub mod distance;
mod error;
mod format;
pub mod ops;
pub mod utils;

pub use error::ProcessError;
pub use format::*;

pub fn process() -> Result<(), ProcessError> {
    let args = cli::Cli::default();

    match args.command.clone() {
        cli::Command::ScaleImage(cmd_args) => ops::scale_image(args, cmd_args)?,
        cli::Command::Brightness(cmd_args) => ops::brightness(args, cmd_args)?,
        cli::Command::Contrast(cmd_args) => ops::contrast(args, cmd_args)?,
        cli::Command::Invert(cmd_args) => ops::invert(args, cmd_args)?,
        cli::Command::RgbaConcat(cmd_args) => ops::rgba_concat(args, cmd_args)?,
        cli::Command::Add(cmd_args) => ops::add(args, cmd_args)?,
        cli::Command::Sub(cmd_args) => ops::sub(args, cmd_args)?,
        cli::Command::Mul(cmd_args) => ops::mul(args, cmd_args)?,
        cli::Command::RenderShapefile(cmd_args) => ops::render_shapefile(args, cmd_args)?,
        cli::Command::FloodFill(cmd_args) => ops::flood_fill(args, cmd_args)?,
        cli::Command::Blur(cmd_args) => ops::blur(args, cmd_args)?,
        cli::Command::MaskMerge(cmd_args) => ops::mask_merge(args, cmd_args)?,
        cli::Command::DistanceField(cmd_args) => distance::run(args, cmd_args)?,
    }
    Ok(())
}
