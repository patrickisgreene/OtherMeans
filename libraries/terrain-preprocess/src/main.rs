use clap::Parser;
use std::env::set_var;
use terrain_preprocess::prelude::*;

fn main() {
    unsafe {
        if true {
            set_var("RAYON_NUM_THREADS", "0");
            set_var("GDAL_NUM_THREADS", "ALL_CPUS");
        } else {
            set_var("RAYON_NUM_THREADS", "1");
            set_var("GDAL_NUM_THREADS", "1");
        }
    }

    let args = Cli::parse();
    let (src_dataset, mut context) = PreprocessContext::from_cli(args).unwrap();

    preprocess(src_dataset, &mut context);
}
