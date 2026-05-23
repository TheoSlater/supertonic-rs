use std::io::Read;
use std::path::PathBuf;

fn main() {
    let target_dir = get_target_dir();
    let lib_path = target_dir.join(native_lib_name());

    if lib_path.exists() {
        println!("cargo:warning=ORT native library exists at {}", lib_path.display());
        return;
    }

    println!("cargo:warning=Downloading ORT native library from NuGet...");

    let url = "https://www.nuget.org/api/v2/package/Microsoft.ML.OnnxRuntime/1.24.2";
    let response = ureq::get(url).call().unwrap_or_else(|e| {
        panic!("Failed to download ORT from {url}: {e}");
    });

    let mut zip_bytes: Vec<u8> = Vec::new();
    response
        .into_body()
        .into_reader()
        .read_to_end(&mut zip_bytes)
        .expect("Failed to read ORT NuGet response body");

    let cursor = std::io::Cursor::new(zip_bytes);
    let mut archive = zip::ZipArchive::new(cursor).expect("Failed to parse ORT NuGet package as ZIP");

    let runtime_path = nuget_runtime_path();
    let mut found = false;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).unwrap();
        if file.name().replace('\\', "/") == runtime_path {
            let mut dll_bytes: Vec<u8> = Vec::new();
            file.read_to_end(&mut dll_bytes)
                .unwrap_or_else(|e| panic!("Failed to read '{}' from ZIP: {e}", file.name()));
            std::fs::write(&lib_path, &dll_bytes)
                .unwrap_or_else(|e| panic!("Failed to write {}: {e}", lib_path.display()));
            println!(
                "cargo:warning=ORT native library written to {} ({} bytes)",
                lib_path.display(),
                dll_bytes.len()
            );
            found = true;
            break;
        }
    }

    if !found {
        panic!(
            "Could not find '{}' in ORT NuGet package ({} entries examined)",
            runtime_path,
            archive.len()
        );
    }
}

fn get_target_dir() -> PathBuf {
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR not set"));
    out_dir
        .ancestors()
        .nth(3)
        .expect("OUT_DIR has fewer than 4 ancestors")
        .to_path_buf()
}

fn native_lib_name() -> String {
    if cfg!(target_os = "windows") {
        "onnxruntime.dll".into()
    } else if cfg!(target_os = "linux") {
        "libonnxruntime.so".into()
    } else if cfg!(target_os = "macos") {
        "libonnxruntime.dylib".into()
    } else {
        panic!("unsupported target OS");
    }
}

fn nuget_runtime_path() -> String {
    let target = std::env::var("TARGET").expect("TARGET not set");
    if target.contains("windows") {
        "runtimes/win-x64/native/onnxruntime.dll"
    } else if target.contains("linux") {
        "runtimes/linux-x64/native/libonnxruntime.so"
    } else if target.contains("apple-darwin") {
        "runtimes/osx-x64/native/libonnxruntime.dylib"
    } else if target.contains("apple-ios") {
        panic!("iOS not supported with load-dynamic");
    } else {
        panic!("unsupported target triple: {target}");
    }
    .into()
}
