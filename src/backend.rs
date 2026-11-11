//! The `Backend` QML singleton: everything the QML side needs from Rust.
//! Structured data crosses the boundary as JSON strings.

use crate::{config, desktop, ipc, library, server, stats};
use core::pin::Pin;
use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::QString;
use serde_json::Value;
use std::os::unix::net::UnixListener;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

/// Set by `main` before the QML engine starts.
pub static START_HIDDEN: AtomicBool = AtomicBool::new(false);
/// No tray icon: a host shell is providing the interface.
pub static HEADLESS: AtomicBool = AtomicBool::new(false);
pub static LISTENER: Mutex<Option<UnixListener>> = Mutex::new(None);

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;

        include!("shim.h");
        /// Creates the QApplication, loads Main.qml and runs the event loop until quit.
        #[rust_name = "run_app"]
        fn undertow_run() -> i32;
    }

    #[auto_cxx_name]
    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qml_singleton]
        #[qproperty(i32, port)]
        #[qproperty(bool, busy)]
        type Backend = super::BackendRust;

        /// JSON array of every installed wallpaper.
        #[qinvokable]
        fn wallpapers(self: &Backend) -> QString;
        #[qinvokable]
        fn current(self: &Backend) -> QString;
        #[qinvokable]
        fn set_current(self: &Backend, id: &QString);
        #[qinvokable]
        fn start_hidden(self: &Backend) -> bool;
        #[qinvokable]
        fn headless(self: &Backend) -> bool;

        /// JSON object of a wallpaper's property values (defaults + saved).
        #[qinvokable]
        fn prop_values(self: &Backend, id: &QString) -> QString;
        #[qinvokable]
        fn set_prop(self: &Backend, id: &QString, name: &QString, value_json: &QString);
        #[qinvokable]
        fn reset_props(self: &Backend, id: &QString);

        #[qinvokable]
        fn set_stats_enabled(self: &Backend, on: bool);
        /// Where the picker thumbnail for a wallpaper is stored.
        #[qinvokable]
        fn thumb_path(self: &Backend, id: &QString) -> QString;
        /// `file://` URL of the thumbnail, or an empty string if none has been captured yet.
        #[qinvokable]
        fn thumb_url(self: &Backend, id: &QString) -> QString;

        // Slow jobs run on a worker thread and report through `job_finished`.
        #[qinvokable]
        fn add(self: Pin<&mut Backend>, spec: &QString);
        #[qinvokable]
        fn update(self: Pin<&mut Backend>, id: &QString);
        #[qinvokable]
        fn remove(self: Pin<&mut Backend>, id: &QString);

        #[qinvokable]
        fn autostart(self: &Backend) -> bool;
        #[qinvokable]
        fn set_autostart(self: &Backend, on: bool);
        #[qinvokable]
        fn launcher(self: &Backend) -> bool;
        #[qinvokable]
        fn set_launcher(self: &Backend, on: bool);

        /// One JSON stats snapshot per second while the feed is enabled.
        #[qsignal]
        fn stats(self: Pin<&mut Backend>, json: QString);
        /// A command line received from the CLI.
        #[qsignal]
        fn command(self: Pin<&mut Backend>, line: QString);
        /// `kind` is "add", "update" or "remove"; `detail` is the new id for a successful add.
        #[qsignal]
        fn job_finished(self: Pin<&mut Backend>, kind: QString, ok: bool, message: QString, detail: QString);
    }

    impl cxx_qt::Threading for Backend {}
    impl cxx_qt::Initialize for Backend {}
}

pub struct BackendRust {
    port: i32,
    busy: bool,
    stats_on: Arc<AtomicBool>,
}

impl Default for BackendRust {
    fn default() -> Self {
        Self { port: 0, busy: false, stats_on: Arc::new(AtomicBool::new(false)) }
    }
}

impl cxx_qt::Initialize for qobject::Backend {
    fn initialize(mut self: Pin<&mut Self>) {
        match server::start() {
            Ok(port) => self.as_mut().set_port(port as i32),
            Err(e) => eprintln!("undertow: could not start the file server: {e}"),
        }

        // CLI commands -> `command` signal on the Qt thread.
        if let Some(listener) = LISTENER.lock().unwrap().take() {
            let qt = self.qt_thread();
            thread::spawn(move || {
                ipc::serve(listener, |line| {
                    let _ = qt.queue(move |backend| backend.command(QString::from(&line)));
                });
            });
        }

        // Stats sampler: idles until a wallpaper that wants the feed is shown.
        let qt = self.qt_thread();
        let enabled = self.rust().stats_on.clone();
        thread::spawn(move || {
            let mut sampler = stats::Sampler::new();
            let mut was_on = false;
            loop {
                let on = enabled.load(Ordering::Relaxed);
                if on && !was_on {
                    sampler.reset();
                    sampler.sample(); // prime the deltas
                    thread::sleep(Duration::from_millis(250));
                }
                was_on = on;
                if on {
                    if let Some(snapshot) = sampler.sample() {
                        let json = snapshot.to_string();
                        if qt.queue(move |backend| backend.stats(QString::from(&json))).is_err() {
                            return; // the QObject is gone, we are shutting down
                        }
                    }
                }
                thread::sleep(Duration::from_secs(1));
            }
        });
    }
}

impl qobject::Backend {
    fn wallpapers(&self) -> QString {
        let list: Vec<Value> = library::all().iter().map(library::Wallpaper::to_json).collect();
        QString::from(&Value::Array(list).to_string())
    }

    fn current(&self) -> QString {
        QString::from(&config::current())
    }

    fn set_current(&self, id: &QString) {
        config::set_current(&id.to_string());
    }

    fn start_hidden(&self) -> bool {
        START_HIDDEN.load(Ordering::Relaxed)
    }

    fn headless(&self) -> bool {
        HEADLESS.load(Ordering::Relaxed)
    }

    fn prop_values(&self, id: &QString) -> QString {
        let values = library::find(&id.to_string()).map(|w| w.prop_values()).unwrap_or_default();
        QString::from(&Value::Object(values).to_string())
    }

    fn set_prop(&self, id: &QString, name: &QString, value_json: &QString) {
        if let Ok(value) = serde_json::from_str::<Value>(&value_json.to_string()) {
            config::set_prop(&id.to_string(), &name.to_string(), value);
        }
    }

    fn reset_props(&self, id: &QString) {
        config::reset_props(&id.to_string());
    }

    fn set_stats_enabled(&self, on: bool) {
        self.rust().stats_on.store(on, Ordering::Relaxed);
    }

    fn thumb_path(&self, id: &QString) -> QString {
        let dir = config::thumb_dir();
        let _ = std::fs::create_dir_all(&dir);
        // Ids are already filesystem-safe; URLs are not, so flatten them.
        let name: String = id.to_string().chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' { c } else { '_' }).collect();
        QString::from(&*dir.join(format!("{name}.png")).to_string_lossy())
    }

    fn thumb_url(&self, id: &QString) -> QString {
        let path = self.thumb_path(id).to_string();
        if std::path::Path::new(&path).is_file() {
            QString::from(&format!("file://{path}"))
        } else {
            QString::default()
        }
    }

    fn run_job(mut self: Pin<&mut Self>, kind: &'static str, job: impl FnOnce() -> Result<(String, String), String> + Send + 'static) {
        if *self.busy() {
            return;
        }
        self.as_mut().set_busy(true);
        let qt = self.qt_thread();
        thread::spawn(move || {
            let result = job();
            let _ = qt.queue(move |mut backend| {
                backend.as_mut().set_busy(false);
                let (ok, message, detail) = match result {
                    Ok((message, detail)) => (true, message, detail),
                    Err(message) => (false, message, String::new()),
                };
                backend.job_finished(QString::from(kind), ok, QString::from(&message), QString::from(&detail));
            });
        });
    }

    fn add(self: Pin<&mut Self>, spec: &QString) {
        let spec = spec.to_string();
        self.run_job("add", move || library::add(&spec).map(|id| (format!("installed {id}"), id)));
    }

    fn update(self: Pin<&mut Self>, id: &QString) {
        let id = id.to_string();
        self.run_job("update", move || library::update(&id).map(|msg| (msg, id)));
    }

    fn remove(self: Pin<&mut Self>, id: &QString) {
        let id = id.to_string();
        self.run_job("remove", move || library::remove(&id).map(|msg| (msg, id)));
    }

    fn autostart(&self) -> bool {
        desktop::autostart_enabled()
    }
    fn set_autostart(&self, on: bool) {
        desktop::set_autostart(on);
    }
    fn launcher(&self) -> bool {
        desktop::launcher_enabled()
    }
    fn set_launcher(&self, on: bool) {
        desktop::set_launcher(on);
    }
}
