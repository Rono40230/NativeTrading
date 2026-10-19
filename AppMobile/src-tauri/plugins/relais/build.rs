fn main() {
    tauri_plugin::Builder::new(&["demarrer", "arreter", "etat"])
        .android_path("android")
        .build();
}
