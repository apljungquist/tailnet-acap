#![expect(
    clippy::unwrap_used,
    reason = "should panic: scripts should exit with a non-zero exit code to halt the build if there is an error"
)]

use std::{env, fs, path, path::Path};

use serde_json::json;

const UDS_PATH: &str = concat!("/run/http/", env!("CARGO_PKG_NAME"), ".sock");

fn generate_manifest(out_dir: &Path) {
    let manifest_out = out_dir.join("manifest.json");

    let content = json!({
        "schemaVersion": "1.5.0",
        "acapPackageConf": {
            "setup": {
                "appName": env!("CARGO_PKG_NAME"),
                "friendlyName": "Tailnet",
                "vendor": "AP Ljungquist",
                "runMode": "respawn",
                "version": env!("CARGO_PKG_VERSION")
            },
            "configuration": {
                "httpConfig": [
                    {
                        "access": "admin",
                        "type": "directory"
                    }
                ],
                "reverseProxy": [
                    {
                        "apiPath": "api",
                        "apiType": "http",
                        "target": format!("unix:{UDS_PATH}"),
                        "access": "admin"
                    }
                ],
                "settingPage": "index.html"
            }
        }
    });
    fs::write(
        manifest_out,
        serde_json::to_string_pretty(&content).unwrap(),
    )
    .unwrap();
}

fn main() {
    println!("cargo:rustc-env=UDS_PATH={UDS_PATH}");
    let out_dir = path::PathBuf::from(env::var("OUT_DIR").unwrap());
    generate_manifest(&out_dir);
}
