use opencore_rustroops::app;

fn main() {
    if let Err(error) = app::run() {
        eprintln!("opencore_rustroops failed to start: {error}");
        std::process::exit(1);
    }
}
