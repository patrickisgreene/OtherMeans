fn main() {
    if let Err(error) = cities::preprocess::run_preprocessor() {
        println!("{error}");
        std::process::exit(1);
    }
}
