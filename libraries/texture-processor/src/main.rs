fn main() {
    if let Err(err) = texture_processor::process() {
        println!("{err:?}");
        std::process::exit(1);
    }
}
