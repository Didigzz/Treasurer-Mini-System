slint::include_modules!();

pub mod app;
pub mod platform;

pub fn run_app() -> anyhow::Result<()> {
    app::run()
}

#[cfg(target_os = "android")]
#[no_mangle]
pub fn android_main(android_app: slint::android::AndroidApp) {
    let internal = android_app.internal_data_path().unwrap_or_default();
    let databases = internal
        .parent()
        .map(|parent| parent.join("databases"))
        .unwrap_or_else(|| internal.clone());
    let backups = android_app
        .external_data_path()
        .unwrap_or_else(|| internal.join("backups"));

    platform::set_android_paths(databases.join("treasurer.db"), backups);
    slint::android::init(android_app).expect("failed to initialize Slint on Android");

    if let Err(error) = run_app() {
        eprintln!("{error:#}");
    }
}
