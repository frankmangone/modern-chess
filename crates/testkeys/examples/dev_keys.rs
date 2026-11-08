//! Writes the development environment files for both apps, with a fresh key pair:
//! the private key goes to social-api's `.env`, the public key to game-api's.
//!
//! Run it with `make dev-keys`. It never overwrites: if the files exist it says so and
//! stops, so a key that is in use is not replaced by accident. Both files are git-ignored.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use modern_chess_testkeys::TestKeys;

fn main() -> ExitCode {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let game_env = root.join("apps/game-api/.env");
    let social_env = root.join("apps/social-api/.env");

    match (game_env.exists(), social_env.exists()) {
        (true, true) => {
            println!("The development .env files already exist; delete both to make new keys.");
            return ExitCode::SUCCESS;
        }
        (false, false) => {}
        _ => {
            eprintln!("Only one of the two .env files exists. Delete it and run this again, so both apps get the same key pair.");
            return ExitCode::FAILURE;
        }
    }

    let keys = TestKeys::generate();
    let written =
        write_env(&game_env, "AUTH_PUBLIC_KEY_PEM", &keys.public_pem, false).and_then(|_| {
            write_env(
                &social_env,
                "SOCIAL_SIGNING_KEY_PEM",
                &keys.private_pem,
                true,
            )
        });

    match written {
        Ok(()) => {
            println!(
                "Wrote apps/game-api/.env (public key) and apps/social-api/.env (private key)."
            );
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("Could not write the .env files: {err}");
            ExitCode::FAILURE
        }
    }
}

/// Copies the app's `.env.example` and adds the key as one quoted line, `\n` for line breaks.
fn write_env(target: &PathBuf, name: &str, pem: &str, private: bool) -> std::io::Result<()> {
    let example = fs::read_to_string(target.with_file_name(".env.example"))?;
    let key_line = pem.lines().collect::<Vec<_>>().join("\\n");
    fs::write(target, format!("{example}{name}=\"{key_line}\\n\"\n"))?;

    #[cfg(unix)]
    if private {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(target, fs::Permissions::from_mode(0o600))?;
    }
    #[cfg(not(unix))]
    let _ = private;
    Ok(())
}
