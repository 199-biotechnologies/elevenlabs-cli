//! `dict download ID` writes the raw PLS XML from the API straight to disk.
//! The response is NOT JSON, so the envelope we print to stdout is a tiny
//! descriptor (`id`, `version_id`, `file`, `bytes`) while the XML goes to
//! the output path.

use assert_cmd::Command as AssertCmd;
use std::io::Write;
use std::path::PathBuf;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn bin() -> AssertCmd {
    AssertCmd::cargo_bin("elevenlabs").unwrap()
}

fn temp_config_with_key(api_key: &str) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    let mut f = std::fs::File::create(&path).unwrap();
    writeln!(f, "api_key = \"{api_key}\"").unwrap();
    (dir, path)
}

const SAMPLE_PLS: &[u8] = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>
<lexicon version=\"1.0\"
         xmlns=\"http://www.w3.org/2005/01/pronunciation-lexicon\"
         alphabet=\"ipa\"
         xml:lang=\"en-US\">
  <lexeme>
    <grapheme>tomato</grapheme>
    <phoneme>təˈmɑːtoʊ</phoneme>
  </lexeme>
</lexicon>"
    .as_bytes();

#[tokio::test(flavor = "multi_thread")]
async fn download_with_explicit_version_writes_bytes_to_output_path() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/pronunciation-dictionaries/pd_dl/v7/download"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_bytes(SAMPLE_PLS)
                .insert_header("content-type", "application/pls+xml"),
        )
        .mount(&mock)
        .await;

    let (_dir, cfg) = temp_config_with_key("sk_test_keyyyyyyyyy");
    let tmp = tempfile::tempdir().unwrap();
    let out_file = tmp.path().join("dict.pls");

    let out = bin()
        .env("ELEVENLABS_CLI_CONFIG", &cfg)
        .env("ELEVENLABS_API_BASE_URL", mock.uri())
        .env_remove("ELEVENLABS_API_KEY")
        .args([
            "dict",
            "download",
            "pd_dl",
            "--version",
            "v7",
            "--output",
            out_file.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(
        out.status.success(),
        "download failed; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );

    // Envelope summary on stdout.
    let body: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(body["status"], "success");
    assert_eq!(body["data"]["id"], "pd_dl");
    assert_eq!(body["data"]["version_id"], "v7");
    assert_eq!(
        body["data"]["bytes"].as_u64().unwrap(),
        SAMPLE_PLS.len() as u64
    );
    assert_eq!(
        body["data"]["file"].as_str().unwrap(),
        out_file.display().to_string()
    );

    // Bytes landed on disk verbatim.
    let disk = std::fs::read(&out_file).unwrap();
    assert_eq!(disk, SAMPLE_PLS);
}

#[tokio::test(flavor = "multi_thread")]
async fn download_without_version_resolves_latest_version_id_first() {
    let mock = MockServer::start().await;
    // 1) `dict show` to learn latest_version_id
    Mock::given(method("GET"))
        .and(path("/v1/pronunciation-dictionaries/pd_auto"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "pd_auto",
            "name": "Auto",
            "latest_version_id": "v42",
        })))
        .mount(&mock)
        .await;
    // 2) XML download at the resolved version
    Mock::given(method("GET"))
        .and(path("/v1/pronunciation-dictionaries/pd_auto/v42/download"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_bytes(SAMPLE_PLS)
                .insert_header("content-type", "application/pls+xml"),
        )
        .mount(&mock)
        .await;

    let (_dir, cfg) = temp_config_with_key("sk_test_keyyyyyyyyy");
    let tmp = tempfile::tempdir().unwrap();
    let out_file = tmp.path().join("auto.pls");

    let out = bin()
        .env("ELEVENLABS_CLI_CONFIG", &cfg)
        .env("ELEVENLABS_API_BASE_URL", mock.uri())
        .env_remove("ELEVENLABS_API_KEY")
        .args([
            "dict",
            "download",
            "pd_auto",
            "-o",
            out_file.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "auto-version download failed; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let body: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(body["data"]["version_id"], "v42");
    let disk = std::fs::read(&out_file).unwrap();
    assert_eq!(disk, SAMPLE_PLS);
}

#[tokio::test(flavor = "multi_thread")]
async fn download_surfaces_api_errors_as_envelope() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(
            "/v1/pronunciation-dictionaries/pd_missing/v1/download",
        ))
        .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
            "detail": { "message": "dictionary not found" }
        })))
        .mount(&mock)
        .await;

    let (_dir, cfg) = temp_config_with_key("sk_test_keyyyyyyyyy");
    let out = bin()
        .env("ELEVENLABS_CLI_CONFIG", &cfg)
        .env("ELEVENLABS_API_BASE_URL", mock.uri())
        .env_remove("ELEVENLABS_API_KEY")
        .args(["dict", "download", "pd_missing", "--version", "v1"])
        .output()
        .unwrap();

    assert!(!out.status.success(), "expected failure on 404");
    // Exit code is 1 (Api, non-401/403/429) per AppError::exit_code.
    assert_eq!(out.status.code(), Some(1));
    let err: serde_json::Value = serde_json::from_slice(&out.stderr).unwrap();
    assert_eq!(err["status"], "error");
    assert_eq!(err["error"]["code"], "api_error");
}
