//! Android install handoff.
//!
//! Copies the verified APK into `MediaStore`'s Downloads collection and opens it
//! with `Intent.ACTION_VIEW`, which starts Android's own installer activity.
//!
//! The copy exists because a `file://` URI is rejected by the installer from
//! Android 7 on, and this app ships no Java/Kotlin class, so it cannot host a
//! `FileProvider`. `MediaStore` is the only system content provider that can
//! serve the bytes, and `FLAG_GRANT_READ_URI_PERMISSION` lets the installer read
//! them without any storage permission.
//!
//! Every JNI call lives in this module; the activity context comes from
//! `ndk-context`, which Slint's Android backend initializes before
//! `android_main` runs.

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

/// `MediaStore.Downloads` exists from API 29 and needs no storage permission.
const MIN_INSTALL_API: i32 = 29;
const MIME_APK: &str = "application/vnd.android.package-archive";
const FLAG_GRANT_READ_URI_PERMISSION: i32 = 0x0000_0001;
const FLAG_ACTIVITY_NEW_TASK: i32 = 0x1000_0000;
const UPDATE_APK_NAME: &str = "boarding-house-treasurer-update.apk";
const CHUNK_BYTES: usize = 1024 * 1024;

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

fn sdk_int(env: &mut JNIEnv<'_>) -> jni::errors::Result<i32> {
    let version = env.find_class("android/os/Build$VERSION")?;
    env.get_static_field(&version, "SDK_INT", "I")?.i()
}

/// API 26+ can report whether this app may install packages. Older devices
/// allow it at install time, so the check is skipped there.
pub fn can_request_package_installs() -> bool {
    with_env(|env, context| {
        if sdk_int(env)? < 26 {
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

/// Hands the downloaded APK to Android's installer. Android shows its own
/// confirmation and only proceeds when the signing key matches the installed
/// app, so nothing here can install silently.
pub fn install_apk(path: &Path) -> Result<(), InstallError> {
    if sdk_int_of_device()? < MIN_INSTALL_API {
        return Err(InstallError::Failed(
            "Installing from inside the app needs Android 10 or newer. Download the update from the release page instead."
                .to_string(),
        ));
    }

    let bytes = std::fs::read(path).map_err(|error| InstallError::Failed(error.to_string()))?;
    if bytes.is_empty() {
        return Err(InstallError::Failed(
            "The downloaded update is empty.".to_string(),
        ));
    }

    with_env(|env, context| hand_off(env, context, &bytes))
}

fn sdk_int_of_device() -> Result<i32, InstallError> {
    with_env(|env, _| sdk_int(env))
}

/// Copies the APK into Downloads, then starts the installer for that content
/// URI. The copy replaces the one left by an earlier attempt, so Downloads
/// never accumulates update files.
fn hand_off(env: &mut JNIEnv<'_>, context: &JObject<'_>, bytes: &[u8]) -> jni::errors::Result<()> {
    let resolver = env
        .call_method(
            context,
            "getContentResolver",
            "()Landroid/content/ContentResolver;",
            &[],
        )?
        .l()?;
    let downloads = env.find_class("android/provider/MediaStore$Downloads")?;
    let collection = env
        .get_static_field(
            &downloads,
            "EXTERNAL_CONTENT_URI",
            "Landroid/net/Uri;",
        )?
        .l()?;

    let selection = env.new_string("_display_name=?")?;
    let name = env.new_string(UPDATE_APK_NAME)?;
    let selection_args = env.new_object_array(1, "java/lang/String", &name)?;
    env.call_method(
        &resolver,
        "delete",
        "(Landroid/net/Uri;Ljava/lang/String;[Ljava/lang/String;)I",
        &[
            JValue::Object(&collection),
            JValue::Object(&selection),
            JValue::Object(&selection_args),
        ],
    )?;

    let values_class = env.find_class("android/content/ContentValues")?;
    let values = env.new_object(&values_class, "()V", &[])?;
    put_string(env, &values, "_display_name", UPDATE_APK_NAME)?;
    put_string(env, &values, "mime_type", MIME_APK)?;
    // Hidden from other apps until the copy is complete.
    put_int(env, &values, "is_pending", 1)?;

    let uri = env
        .call_method(
            &resolver,
            "insert",
            "(Landroid/net/Uri;Landroid/content/ContentValues;)Landroid/net/Uri;",
            &[JValue::Object(&collection), JValue::Object(&values)],
        )?
        .l()?;
    if uri.is_null() {
        return Err(jni::errors::Error::NullPtr("MediaStore insert"));
    }

    let result = write_copy(env, &resolver, &uri, bytes)
        .and_then(|()| publish(env, &resolver, &uri))
        .and_then(|()| open_installer(env, context, &uri));
    if result.is_err() {
        abandon(env, &resolver, &uri);
    }
    result
}

fn write_copy(
    env: &mut JNIEnv<'_>,
    resolver: &JObject<'_>,
    uri: &JObject<'_>,
    bytes: &[u8],
) -> jni::errors::Result<()> {
    let stream = env
        .call_method(
            resolver,
            "openOutputStream",
            "(Landroid/net/Uri;)Ljava/io/OutputStream;",
            &[JValue::Object(uri)],
        )?
        .l()?;
    if stream.is_null() {
        return Err(jni::errors::Error::NullPtr("MediaStore output stream"));
    }
    for chunk in bytes.chunks(CHUNK_BYTES) {
        let array: JObject = env.byte_array_from_slice(chunk)?.into();
        env.call_method(&stream, "write", "([B)V", &[JValue::Object(&array)])?;
    }
    env.call_method(&stream, "flush", "()V", &[])?;
    env.call_method(&stream, "close", "()V", &[])?;
    Ok(())
}

/// Clears `is_pending`, which makes the copy readable by the installer.
fn publish(env: &mut JNIEnv<'_>, resolver: &JObject<'_>, uri: &JObject<'_>) -> jni::errors::Result<()> {
    let values_class = env.find_class("android/content/ContentValues")?;
    let values = env.new_object(&values_class, "()V", &[])?;
    put_int(env, &values, "is_pending", 0)?;
    let null = JObject::null();
    env.call_method(
        resolver,
        "update",
        "(Landroid/net/Uri;Landroid/content/ContentValues;Ljava/lang/String;[Ljava/lang/String;)I",
        &[
            JValue::Object(uri),
            JValue::Object(&values),
            JValue::Object(&null),
            JValue::Object(&null),
        ],
    )?;
    Ok(())
}

fn open_installer(
    env: &mut JNIEnv<'_>,
    context: &JObject<'_>,
    uri: &JObject<'_>,
) -> jni::errors::Result<()> {
    let intent_class = env.find_class("android/content/Intent")?;
    let action = env
        .get_static_field(&intent_class, "ACTION_VIEW", "Ljava/lang/String;")?
        .l()?;
    let intent = env.new_object(
        &intent_class,
        "(Ljava/lang/String;)V",
        &[JValue::Object(&action)],
    )?;
    let mime = env.new_string(MIME_APK)?;
    env.call_method(
        &intent,
        "setDataAndType",
        "(Landroid/net/Uri;Ljava/lang/String;)Landroid/content/Intent;",
        &[JValue::Object(uri), JValue::Object(&mime)],
    )?;
    env.call_method(
        &intent,
        "addFlags",
        "(I)Landroid/content/Intent;",
        &[JValue::Int(
            FLAG_GRANT_READ_URI_PERMISSION | FLAG_ACTIVITY_NEW_TASK,
        )],
    )?;
    env.call_method(
        context,
        "startActivity",
        "(Landroid/content/Intent;)V",
        &[JValue::Object(&intent)],
    )?;
    Ok(())
}

/// Best effort: an abandoned row must never block the next attempt.
fn abandon(env: &mut JNIEnv<'_>, resolver: &JObject<'_>, uri: &JObject<'_>) {
    let null = JObject::null();
    let _ = env.call_method(
        resolver,
        "delete",
        "(Landroid/net/Uri;Ljava/lang/String;[Ljava/lang/String;)I",
        &[
            JValue::Object(uri),
            JValue::Object(&null),
            JValue::Object(&null),
        ],
    );
}

fn put_string(
    env: &mut JNIEnv<'_>,
    values: &JObject<'_>,
    key: &str,
    value: &str,
) -> jni::errors::Result<()> {
    let key = env.new_string(key)?;
    let value = env.new_string(value)?;
    env.call_method(
        values,
        "put",
        "(Ljava/lang/String;Ljava/lang/String;)V",
        &[JValue::Object(&key), JValue::Object(&value)],
    )?;
    Ok(())
}

fn put_int(
    env: &mut JNIEnv<'_>,
    values: &JObject<'_>,
    key: &str,
    value: i32,
) -> jni::errors::Result<()> {
    let key = env.new_string(key)?;
    let boxed = env.new_object("java/lang/Integer", "(I)V", &[JValue::Int(value)])?;
    env.call_method(
        values,
        "put",
        "(Ljava/lang/String;Ljava/lang/Integer;)V",
        &[JValue::Object(&key), JValue::Object(&boxed)],
    )?;
    Ok(())
}
