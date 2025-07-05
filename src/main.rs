mod app;

use app::App;

fn main() {
    console_log::init_with_level(log::Level::Debug).expect("console logging failed");
    log::info!("console logging succeeded");
    yew::Renderer::<App>::new().render();
}
