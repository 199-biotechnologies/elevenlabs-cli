# Security Policy

## Reporting a Vulnerability

Please do **not** report security vulnerabilities through public GitHub
issues.

Instead, email `security@199bio.com` with:

- A description of the issue
- Steps to reproduce
- The version of `elevenlabs-cli` you're running (`elevenlabs --version`)
- Any logs or output (with secrets redacted)

We will acknowledge your report within 72 hours and work with you on a
coordinated disclosure timeline.

## Scope

In scope:

- Vulnerabilities in the `elevenlabs-cli` Rust binary and its published
  crates.
- Vulnerabilities in the install scripts, CI workflows, and release
  pipeline.

Out of scope:

- Vulnerabilities in the ElevenLabs API itself — please report those
  directly to ElevenLabs at <https://elevenlabs.io/security>.
- Issues that require attacker access to your local machine or API key.

## Supported Versions

The latest minor version on the `main` branch receives security fixes.
Older versions are best-effort.

| Version | Supported |
|---|---|
| 0.1.x | ✅ |

## Secret Handling

This CLI never logs, prints, or otherwise echoes your `ELEVENLABS_API_KEY`
in plain text. If you find a code path where a secret leaks, treat it
as a security issue and report it privately.
