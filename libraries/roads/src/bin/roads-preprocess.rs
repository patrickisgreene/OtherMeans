fn main() {
    if let Err(error) = roads::preprocess::run_preprocessor() {
        println!("{error}");
        std::process::exit(1);
    }
}
