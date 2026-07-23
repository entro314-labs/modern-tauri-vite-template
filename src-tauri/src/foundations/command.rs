//! Process spawn helper with redaction-aware logging and structured errors.
//! See `docs/LUDUSAVI_PATTERNS.md` section 7.

use itertools::Itertools;

pub use crate::foundations::error::CommandError;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Privacy {
  Public,
  /// Args are redacted from logs and from CommandError (use for tokens, sensitive paths).
  Private,
}

impl Privacy {
  pub fn sensitive(&self) -> bool {
    *self == Self::Private
  }
}

pub struct CommandOutput {
  pub code: i32,
  pub stdout: String,
  pub stderr: String,
}

/// Run a command synchronously. `success` lists the exit codes treated as success.
///
/// On Windows, sets `CREATE_NO_WINDOW` so spawned children don't flash a console.
pub fn run_command(
  executable: &str,
  args: &[&str],
  success: &[i32],
  privacy: Privacy,
) -> Result<CommandOutput, CommandError> {
  let mut command = std::process::Command::new(executable);
  command
    .stdout(std::process::Stdio::piped())
    .stderr(std::process::Stdio::piped())
    .args(args);

  #[cfg(target_os = "windows")]
  {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    command.creation_flags(CREATE_NO_WINDOW);
  }

  let collect_args = || {
    if privacy.sensitive() {
      vec!["**REDACTED**".to_string()]
    } else {
      args.iter().map(|x| x.to_string()).collect::<Vec<_>>()
    }
  };
  let format_args = || {
    if privacy.sensitive() {
      "**REDACTED**".to_string()
    } else {
      args.iter().map(|x| x.to_string()).join(" ")
    }
  };

  log::debug!("Running command: {} {:?}", executable, collect_args());

  match command.output() {
    Ok(output) => match output.status.code() {
      Some(code) if success.contains(&code) => {
        log::debug!(
          "Command succeeded with {}: {} {}",
          code,
          executable,
          format_args()
        );

        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

        Ok(CommandOutput {
          code,
          stdout,
          stderr,
        })
      }
      Some(code) => {
        log::error!(
          "Command failed with {}: {} {}",
          code,
          executable,
          format_args()
        );

        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        log::error!("Command stdout: {stdout}");
        log::error!("Command stderr: {stderr}");

        Err(CommandError::Exited {
          program: executable.to_string(),
          args: collect_args(),
          code,
          stdout: (!stdout.is_empty()).then_some(stdout),
          stderr: (!stderr.is_empty()).then_some(stderr),
        })
      }
      None => {
        log::warn!("Command terminated: {} {}", executable, format_args());
        Err(CommandError::Terminated {
          program: executable.to_string(),
          args: collect_args(),
        })
      }
    },
    Err(error) => {
      log::warn!("Command did not launch: {} {}", executable, format_args());
      Err(CommandError::Launched {
        program: executable.to_string(),
        args: collect_args(),
        raw: error.to_string(),
      })
    }
  }
}
