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
                    Ok(Box::new(super::Playground::default()))
                }),
            )
            .await;
        match result {
            Ok(()) => status(
                "Local browser app · session-only history · large plots may briefly pause interaction",
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
