//! Minimal local-browser bootstrap; native startup stays in main.rs.
use wasm_bindgen::JsCast;

fn status(message: &str) {
    if let Some(element) = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.get_element_by_id("status"))
    {
        element.set_text_content(Some(message));
    }
}

const HISTORY_KEY: &str = "math-playground.history.v1";
struct LocalHistory;
impl math_playground::history::BrowserStorage for LocalHistory {
    fn read(&self) -> Result<Option<String>, String> {
        storage()?
            .get_item(HISTORY_KEY)
            .map_err(|_| "Browser history could not be read.".into())
    }
    fn write(&self, value: &str) -> Result<(), String> {
        storage()?
            .set_item(HISTORY_KEY, value)
            .map_err(|_| "Browser storage is full or blocked.".into())
    }
}
fn storage() -> Result<web_sys::Storage, String> {
    web_sys::window()
        .ok_or("Browser window unavailable.")?
        .local_storage()
        .map_err(|_| "Browser storage is blocked.".to_owned())?
        .ok_or("Browser storage unavailable.".into())
}

pub fn launch() {
    wasm_bindgen_futures::spawn_local(async {
        let canvas = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.get_element_by_id("playground"))
            .and_then(|e| e.dyn_into::<web_sys::HtmlCanvasElement>().ok());
        let Some(canvas) = canvas else {
            status("Startup failed: canvas not found. Check browser developer console.");
            return;
        };
        let runner = eframe::WebRunner::new();
        let result = runner
            .start(
                canvas,
                eframe::WebOptions::default(),
                Box::new(|cc| {
                    cc.egui_ctx.set_theme(eframe::egui::ThemePreference::Dark);
                    Ok(Box::new(super::Playground {
                        history: math_playground::history::Store::browser(Box::new(LocalHistory)),
                        ..Default::default()
                    }))
                }),
            )
            .await;
        match result {
            Ok(()) => status(
                "Browser app · browser-local history (see History for save status) · large plots may briefly pause interaction",
            ),
            Err(error) => {
                web_sys::console::error_1(&error);
                status(
                    "Startup failed: graphics initialization error. Check browser developer console.",
                );
            }
        }
    });
}
