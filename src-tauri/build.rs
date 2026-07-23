fn main() {
  println!("cargo:rerun-if-env-changed=TAURI_APP_NAME");
  println!("cargo:rerun-if-env-changed=TAURI_APP_VERSION");

  #[cfg(target_os = "windows")]
  {
    println!("cargo:rerun-if-changed=assets/windows/manifest.rc");
    println!("cargo:rerun-if-changed=assets/windows/manifest.xml");

    // Tauri's own manifest already covers WebView DPI for the host binary, but
    // this manifest adds `longPathAware`, `permonitorv2`, and Common Controls v6
    // which are not in Tauri's default manifest. Safe to combine.
    if let Err(e) = embed_resource::compile("assets/windows/manifest.rc", embed_resource::NONE)
      .manifest_optional()
    {
      println!("cargo:warning=failed to embed Windows manifest: {e}");
    }
  }

  if let Err(error) = tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
    tauri_build::AppManifest::new().commands(&[
      "greet",
      "get_app_info",
      "get_app_dir",
      "get_os",
      "resolve_path",
      "check_for_updates",
      "download_update",
      "update_install_support",
      "update_staged",
    ]),
  )) {
    panic!("failed to run tauri build script: {error}");
  }
}
