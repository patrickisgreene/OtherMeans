fn main() {
    if let Err(error) = shipping_lanes::preprocess::run_preprocessor() {
        println!("{error}");
        std::process::exit(1);
    }
}
