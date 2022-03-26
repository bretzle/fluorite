fn main() {
    if cfg!(windows) {
        let mut res = winres::WindowsResource::new();
        res.set_icon("../assets/fluorite.ico");
        res.compile().unwrap();
    }
}
