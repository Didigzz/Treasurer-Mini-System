//! Android install handoff.
//!
//! Uses a `PackageInstaller` session so no Java/Kotlin class is needed. Every
//! JNI call lives in this module; the activity context comes from `ndk-context`,
//! which Slint's Android backend initializes before `android_main` runs.

use std::path::Path;

use jni::objects::{JObject, JString, JValue};
use jni::sys::jobject;
use jni::{JNIEnv, JavaVM};

#[derive(Debug)]
pub enum InstallError {
    /// "Install unknown apps" is not granted for this app.
    Permission,
    /// Any other JNI or installer failure, with Android's own text.
    Failed(String),
}

const MODE_FULL_INSTALL: i32 = 1;
const FLAG_MUTABLE: i32 = 0x0200_0000;
const FLAG_UPDATE_CURRENT: i32 = 0x0800_0000;

fn java_vm() -> Result<JavaVM, InstallError> {
    let context = ndk_context::android_context();
    unsafe { JavaVM::from_raw(context.vm().cast::<jni::sys::JavaVM>()) }
        .map_err(|error| InstallError::Failed(error.to_string()))
}

fn with_env<T>(
    action: impl FnOnce(&mut JNIEnv<'_>, &JObject<'_>) -> jni::errors::Result<T>,
) -> Result<T, InstallError> {
    let vm = java_vm()?;
    let mut env = vm
        .attach_current_thread()
        .map_err(|error| InstallError::Failed(error.to_string()))?;
    let context = unsafe {
        JObject::from_raw(ndk_context::android_context().context().cast::<jobject>() as jobject)
    };
    action(&mut env, &context).map_err(|error| InstallError::Failed(error.to_string()))
}

/// API 26+ can report whether this app may install packages. Older devices
/// allow it at install time, so the check is skipped there.
pub fn can_request_package_installs() -> bool {
    with_env(|env, context| {
        let version = env.find_class("android/os/Build$VERSION")?;
        let sdk = env.get_static_field(&version, "SDK_INT", "I")?.i()?;
        if sdk < 26 {
            return Ok(true);
        }
        let manager = env.call_method(
            context,
            "getPackageManager",
            "()Landroid/content/pm/PackageManager;",
            &[],
        )?;
        let manager = manager.l()?;
        env.call_method(&manager, "canRequestPackageInstalls", "()Z", &[])?
            .z()
    })
    .unwrap_or(false)
}

/// Opens this app's "Install unknown apps" screen, falling back to the app list
/// in system settings when the device does not resolve the deeper link.
pub fn open_install_settings() -> Result<(), InstallError> {
    with_env(|env, context| {
        let package = env
            .call_method(context, "getPackageName", "()Ljava/lang/String;", &[])?
            .l()?;
        let package = JString::from(package);
        let package_name = env.get_string(&package)?.to_string_lossy().to_string();

        let settings = env.find_class("android/provider/Settings")?;
        let action = env
            .get_static_field(
                &settings,
                "ACTION_MANAGE_UNKNOWN_APP_SOURCES",
                "Ljava/lang/String;",
            )?
            .l()?;
        let uri_class = env.find_class("android/net/Uri")?;
        let uri_text = env.new_string(format!("package:{package_name}"))?;
        let uri = env
            .call_static_method(
                &uri_class,
                "parse",
                "(Ljava/lang/String;)Landroid/net/Uri;",
                &[JValue::Object(&uri_text)],
            )?
            .l()?;

        let intent_class = env.find_class("android/content/Intent")?;
        let intent = env.new_object(
            &intent_class,
            "(Ljava/lang/String;Landroid/net/Uri;)V",
            &[JValue::Object(&action), JValue::Object(&uri)],
        )?;
        let new_task = env
            .get_static_field(&intent_class, "FLAG_ACTIVITY_NEW_TASK", "I")?
            .i()?;
        env.call_method(
            &intent,
            "addFlags",
            "(I)Landroid/content/Intent;",
            &[JValue::Int(new_task)],
        )?;

        let started = env.call_method(
            context,
            "startActivity",
            "(Landroid/content/Intent;)V",
            &[JValue::Object(&intent)],
        );
        if started.is_err() || env.exception_check()? {
            env.exception_clear()?;
            let fallback = env
                .get_static_field(
                    &settings,
                    "ACTION_MANAGE_APPLICATIONS_SETTINGS",
                    "Ljava/lang/String;",
                )?
                .l()?;
            let intent = env.new_object(
                &intent_class,
                "(Ljava/lang/String;)V",
                &[JValue::Object(&fallback)],
            )?;
            env.call_method(
                &intent,
                "addFlags",
                "(I)Landroid/content/Intent;",
                &[JValue::Int(new_task)],
            )?;
            env.call_method(
                context,
                "startActivity",
                "(Landroid/content/Intent;)V",
                &[JValue::Object(&intent)],
            )?;
        }
        Ok(())
    })
}

/// Hands the downloaded APK to a `PackageInstaller` session. Android always
/// shows its own confirmation and only proceeds when the signing key matches
/// the installed app.
pub fn install_apk(path: &Path) -> Result<(), InstallError> {
    let bytes = std::fs::read(path).map_err(|error| InstallError::Failed(error.to_string()))?;
    if bytes.is_empty() {
        return Err(InstallError::Failed(
            "The downloaded update is empty.".to_string(),
        ));
    }

    with_env(|env, context| {
        let manager = env
            .call_method(
                context,
                "getPackageManager",
                "()Landroid/content/pm/PackageManager;",
                &[],
            )?
            .l()?;
        let installer = env
            .call_method(
                &manager,
                "getPackageInstaller",
                "()Landroid/content/pm/PackageInstaller;",
                &[],
            )?
            .l()?;

        let params_class = env.find_class("android/content/pm/PackageInstaller$SessionParams")?;
        let params = env.new_object(
            &params_class,
            "(I)V",
            &[JValue::Int(MODE_FULL_INSTALL)],
        )?;
        let session_id = env
            .call_method(
                &installer,
                "createSession",
                "(Landroid/content/pm/PackageInstaller$SessionParams;)I",
                &[JValue::Object(&params)],
            )?
            .i()?;
        let session = env
            .call_method(
                &installer,
                "openSession",
                "(I)Landroid/content/pm/PackageInstaller$Session;",
                &[JValue::Int(session_id)],
            )?
            .l()?;

        let result = write_and_commit(env, context, &session, &bytes);
        let _ = env.call_method(&session, "close", "()V", &[]);
        result
    })
}

fn write_and_commit(
    env: &mut JNIEnv<'_>,
    context: &JObject<'_>,
    session: &JObject<'_>,
    bytes: &[u8],
) -> jni::errors::Result<()> {
    let name = env.new_string("package")?;
    let stream = env
        .call_method(
            session,
            "openWrite",
            "(Ljava/lang/String;JJ)Ljava/io/OutputStream;",
            &[JValue::Object(&name), JValue::Long(0), JValue::Long(-1)],
        )?
        .l()?;
    let array: JObject = env.byte_array_from_slice(bytes)?.into();
    env.call_method(&stream, "write", "([B)V", &[JValue::Object(&array)])?;
    env.call_method(
        session,
        "fsync",
        "(Ljava/io/OutputStream;)V",
        &[JValue::Object(&stream)],
    )?;
    env.call_method(&stream, "close", "()V", &[])?;

    let intent_class = env.find_class("android/content/Intent")?;
    let activity_class = env
        .call_method(context, "getClass", "()Ljava/lang/Class;", &[])?
        .l()?;
    let intent = env.new_object(
        &intent_class,
        "(Landroid/content/Context;Ljava/lang/Class;)V",
        &[JValue::Object(context), JValue::Object(&activity_class)],
    )?;

    let pending_class = env.find_class("android/app/PendingIntent")?;
    let pending = env
        .call_static_method(
            pending_class,
            "getActivity",
            "(Landroid/content/Context;ILandroid/content/Intent;I)Landroid/app/PendingIntent;",
            &[
                JValue::Object(context),
                JValue::Int(0),
                JValue::Object(&intent),
                JValue::Int(FLAG_MUTABLE | FLAG_UPDATE_CURRENT),
            ],
        )?
        .l()?;
    let sender = env
        .call_method(&pending, "getIntentSender", "()Landroid/content/IntentSender;", &[])?
        .l()?;
    env.call_method(
        session,
        "commit",
        "(Landroid/content/IntentSender;)V",
        &[JValue::Object(&sender)],
    )?;
    Ok(())
}
