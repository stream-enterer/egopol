fn main() {
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

    // 2. Register static plugin resolver
    emcore::emFpPlugin::set_static_plugin_resolver(
        emMain::static_plugins::resolve_static_plugin,
    );

    // 3. Try IPC client (unless -noclient)
    if !no_client {
        let server_name = emMain::emMain::CalcServerName();
        if emMain::emMain::try_ipc_client(&server_name, visit.as_deref()) {
            // Another instance handled the request; exit.
            return;
        }
    }

    // 4. Start GUI framework
    let config = emMain::emMainWindow::emMainWindowConfig {
        fullscreen,
        visit,
        ..Default::default()
    };

    let setup = Box::new(
        move |app: &mut emcore::emGUIFramework::App,
              event_loop: &winit::event_loop::ActiveEventLoop| {
            emMain::emMainWindow::create_main_window(app, event_loop, &config);
        },
    );

    emcore::emGUIFramework::App::new(setup).run();
}
