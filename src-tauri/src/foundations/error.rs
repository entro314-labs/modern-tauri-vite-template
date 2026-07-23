use std::fmt;

pub type AnyError = Box<dyn std::error::Error + Send + Sync>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CommandError {
  Launched {
    program: String,
    args: Vec<String>,
    raw: String,
  },
  Terminated {
    program: String,
    args: Vec<String>,
  },
  Exited {
    program: String,
    args: Vec<String>,
    code: i32,
    stdout: Option<String>,
    stderr: Option<String>,
  },
}

impl CommandError {
  pub fn command(&self) -> String {
    let (program, args) = match self {
      Self::Launched { program, args, .. } => (program, args),
      Self::Terminated { program, args } => (program, args),
      Self::Exited { program, args, .. } => (program, args),
    };

    let program = quote(program);
    let args: Vec<String> = args.iter().map(quote).collect();
    format!("{} {}", program, args.join(" "))
  }
}

impl fmt::Display for CommandError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::Launched { raw, .. } => write!(f, "failed to launch `{}`: {}", self.command(), raw),
      Self::Terminated { .. } => write!(f, "process terminated: `{}`", self.command()),
      Self::Exited { code, stderr, .. } => write!(
        f,
        "process `{}` exited with {}{}",
        self.command(),
        code,
        stderr
          .as_ref()
          .map(|s| format!(": {s}"))
          .unwrap_or_default()
      ),
    }
  }
}

impl std::error::Error for CommandError {}

fn quote(text: impl AsRef<str>) -> String {
  let text = text.as_ref();
  if text.contains(' ') {
    format!("\"{text}\"")
  } else {
    text.to_owned()
  }
}
