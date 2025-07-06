mod app;

use app::App;

fn main() {
    console_log::init_with_level(log::Level::Debug).expect("console logging failed");
    yew::Renderer::<App>::new().render();
}
