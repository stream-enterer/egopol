fn main() {
    // Enable dlog! tracing if EMCORE_DLOG=1 is set in the environment.
    // Mirrors Eagle Mode C++ `emEnableDLog` toggled by `--dlog` CLI flag;
    // env var chosen here so debug-instrumented runs need no flag plumbing.
    if std::env::var("EMCORE_DLOG").is_ok_and(|v| v == "1") {
        emcore::emStd1::emEnableDLog(true);
    }

    // 1. Parse CLI args (simplified)
    let args: Vec<String> = std::env::args().collect();
    let mut fullscreen = false;
    let mut visit: Option<String> = None;
    let mut no_client = false;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-fullscreen" => fullscreen = true,
            "-noclient" => no_client = true,
            "-noserver" => { /* reserved for future IPC server disable */ }
            "-visit" => {
                i += 1;
                if i < args.len() {
                    visit = Some(args[i].clone());
                }
            }
            _ => {}
        }
        i += 1;
    }

    // 2. Try IPC client (unless -noclient)
    if !no_client {
        let server_name = emMain::emMain::CalcServerName();
        if emMain::emMain::try_ipc_client(&server_name, visit.as_deref()) {
            // Another instance handled the request; exit.
            return;
        }
    }

    // 3. Start GUI framework
    let config = emMain::emMainWindow::emMainWindowConfig {
        fullscreen,
        visit,
        ..Default::default()
    };

    let setup = Box::new(
        move |app: &mut emcore::emGUIFramework::App,
              event_loop: &winit::event_loop::ActiveEventLoop| {
            let mw = emMain::emMainWindow::create_main_window(app, event_loop, config);
            emMain::emMainWindow::set_main_window(mw);
        },
    );

    emcore::emGUIFramework::App::new(setup).run();
}
