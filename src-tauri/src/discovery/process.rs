use crate::discovery::compatibility;
use crate::domain::{CancellationState, CommandError, ProcessEvidence, PromptState};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandPlan {
    pub executable: PathBuf,
    pub arguments: Vec<String>,
    pub working_directory: PathBuf,
}

pub fn command_plan(
    executable: impl Into<PathBuf>,
    root: impl Into<PathBuf>,
) -> Result<CommandPlan, CommandError> {
    let executable = executable.into();
    let root = root.into();
    if !executable.is_absolute() {
        return Err(CommandError::new(
            "executable_not_absolute",
            "Packwiz executable must be absolute",
        ));
    }
    if !root.is_absolute() {
        return Err(CommandError::new(
            "root_not_absolute",
            "Project root must be absolute",
        ));
    }
    Ok(CommandPlan {
        executable,
        arguments: compatibility::tested_profile().command,
        working_directory: root,
    })
}

#[derive(Debug)]
pub struct PromptMachine {
    pub state: PromptState,
    pub cancellation: CancellationState,
}

impl Default for PromptMachine {
    fn default() -> Self {
        Self {
            state: PromptState::NotSeen,
            cancellation: CancellationState::NotAttempted,
        }
    }
}

impl PromptMachine {
    pub fn observe(&mut self, output: &str) -> bool {
        self.state = compatibility::classify_prompt(output);
        matches!(self.state, PromptState::ExpectedSeen)
    }

    pub fn authorize_n(&mut self) -> Result<Vec<u8>, CommandError> {
        if !matches!(self.state, PromptState::ExpectedSeen) {
            return Err(CommandError::new(
                "prompt_not_authorized",
                "The expected Packwiz prompt was not detected",
            ));
        }
        self.cancellation = CancellationState::SentN;
        Ok(compatibility::CANCELLATION_RESPONSE.as_bytes().to_vec())
    }
}

#[derive(Debug, Clone)]
pub struct OperationCoordinator {
    active_project: Arc<Mutex<Option<String>>>,
}

impl Default for OperationCoordinator {
    fn default() -> Self {
        Self {
            active_project: Arc::new(Mutex::new(None)),
        }
    }
}

impl OperationCoordinator {
    pub fn acquire(&self, project_id: &str) -> Result<OperationLease, CommandError> {
        let mut active = self.active_project.lock().map_err(|_| {
            CommandError::new(
                "operation_unavailable",
                "Operation coordinator is unavailable",
            )
        })?;
        if active.as_deref() == Some(project_id) {
            return Err(CommandError::new(
                "operation_already_running",
                "Discovery is already running for this project",
            ));
        }
        *active = Some(project_id.to_string());
        Ok(OperationLease {
            project_id: project_id.to_string(),
            active: Arc::clone(&self.active_project),
        })
    }
}

pub struct OperationLease {
    project_id: String,
    active: Arc<Mutex<Option<String>>>,
}
impl Drop for OperationLease {
    fn drop(&mut self) {
        if let Ok(mut active) = self.active.lock() {
            if active.as_deref() == Some(&self.project_id) {
                *active = None;
            }
        }
    }
}

pub fn empty_evidence(plan: &CommandPlan, root: &Path) -> ProcessEvidence {
    ProcessEvidence {
        executable: plan.executable.to_string_lossy().into_owned(),
        arguments: plan.arguments.clone(),
        working_directory: root.to_string_lossy().into_owned(),
        stdout: String::new(),
        stderr: String::new(),
        exit_code: None,
        started_at: String::new(),
        finished_at: None,
        output_truncated: false,
        prompt: PromptState::NotSeen,
        cancellation: CancellationState::NotAttempted,
    }
}

#[derive(Debug, Clone)]
pub struct RunLimits {
    pub max_output_bytes: usize,
    pub timeout: Duration,
    pub cancel: Option<Arc<AtomicBool>>,
}

impl Default for RunLimits {
    fn default() -> Self {
        Self {
            max_output_bytes: compatibility::MAX_OUTPUT_BYTES as usize,
            timeout: Duration::from_secs(compatibility::TIMEOUT_SECONDS),
            cancel: None,
        }
    }
}

enum OutputChunk {
    Stdout(Vec<u8>),
    Stderr(Vec<u8>),
    Closed,
}

pub fn run_probe(plan: &CommandPlan, limits: RunLimits) -> Result<ProcessEvidence, CommandError> {
    let started_at = timestamp();
    eprintln!(
        "[discovery] spawning executable={} args={:?} cwd={}",
        plan.executable.display(),
        plan.arguments,
        plan.working_directory.display()
    );
    let mut child = Command::new(&plan.executable)
        .args(&plan.arguments)
        .current_dir(&plan.working_directory)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| {
            CommandError::new("process_spawn_failed", "Packwiz could not be started")
                .with_details(error.to_string())
        })?;
    eprintln!("[discovery] child spawned pid={}", child.id());
    let mut stdin = Some(child.stdin.take().ok_or_else(|| {
        CommandError::new(
            "process_stdin_unavailable",
            "Packwiz stdin could not be opened",
        )
    })?);
    let stdout = child.stdout.take().ok_or_else(|| {
        CommandError::new(
            "process_stdout_unavailable",
            "Packwiz stdout could not be opened",
        )
    })?;
    let stderr = child.stderr.take().ok_or_else(|| {
        CommandError::new(
            "process_stderr_unavailable",
            "Packwiz stderr could not be opened",
        )
    })?;
    let (sender, receiver) = mpsc::channel();
    spawn_reader(stdout, OutputKind::Stdout, sender.clone());
    spawn_reader(stderr, OutputKind::Stderr, sender);
    let deadline = std::time::Instant::now() + limits.timeout;
    let mut machine = PromptMachine::default();
    let mut stdout_text = String::new();
    let mut stderr_text = String::new();
    let mut output_truncated = false;
    let mut closed_readers = 0;
    let mut sent_n = false;
    let mut exited_at = None;
    loop {
        if limits
            .cancel
            .as_ref()
            .is_some_and(|flag| flag.load(Ordering::Relaxed))
        {
            eprintln!(
                "[discovery] cancellation flag observed; terminating pid={}",
                child.id()
            );
            terminate_child(&mut child);
            return Ok(ProcessEvidence {
                executable: plan.executable.to_string_lossy().into_owned(),
                arguments: plan.arguments.clone(),
                working_directory: plan.working_directory.to_string_lossy().into_owned(),
                stdout: stdout_text,
                stderr: stderr_text,
                exit_code: None,
                started_at,
                finished_at: Some(timestamp()),
                output_truncated,
                prompt: machine.state,
                cancellation: CancellationState::UserCancelled,
            });
        }
        if let Some(status) = child.try_wait().map_err(|error| {
            CommandError::new("process_wait_failed", "Packwiz status could not be read")
                .with_details(error.to_string())
        })? {
            if exited_at.is_none() {
                exited_at = Some(std::time::Instant::now());
                eprintln!("[discovery] child exited status={status:?}; draining output");
            }
            if closed_readers >= 2
                || exited_at.is_some_and(|started| started.elapsed() >= Duration::from_millis(250))
            {
                eprintln!(
                    "[discovery] returning after child exit closed_readers={} stdout_bytes={} stderr_bytes={}",
                    closed_readers,
                    stdout_text.len(),
                    stderr_text.len()
                );
                let combined_output = combined_output(&stdout_text, &stderr_text);
                let cancellation = compatibility::classify_cancellation(
                    &combined_output,
                    status.code(),
                    &machine.state,
                );
                return Ok(ProcessEvidence {
                    executable: plan.executable.to_string_lossy().into_owned(),
                    arguments: plan.arguments.clone(),
                    working_directory: plan.working_directory.to_string_lossy().into_owned(),
                    stdout: stdout_text,
                    stderr: stderr_text,
                    exit_code: status.code(),
                    started_at,
                    finished_at: Some(timestamp()),
                    output_truncated,
                    prompt: machine.state,
                    cancellation,
                });
            }
        }
        if std::time::Instant::now() >= deadline {
            eprintln!(
                "[discovery] timeout reached; terminating pid={}",
                child.id()
            );
            terminate_child(&mut child);
            return Ok(ProcessEvidence {
                executable: plan.executable.to_string_lossy().into_owned(),
                arguments: plan.arguments.clone(),
                working_directory: plan.working_directory.to_string_lossy().into_owned(),
                stdout: stdout_text,
                stderr: stderr_text,
                exit_code: None,
                started_at,
                finished_at: Some(timestamp()),
                output_truncated,
                prompt: machine.state,
                cancellation: CancellationState::Terminated,
            });
        }
        match receiver.recv_timeout(Duration::from_millis(25)) {
            Ok(OutputChunk::Stdout(bytes)) => {
                append_bounded(
                    &mut stdout_text,
                    &bytes,
                    limits.max_output_bytes,
                    &mut output_truncated,
                );
                if !sent_n && machine.observe(&combined_output(&stdout_text, &stderr_text)) {
                    eprintln!("[discovery] expected prompt observed on stdout; sending n and closing stdin");
                    let response = machine.authorize_n()?;
                    stdin
                        .as_mut()
                        .expect("stdin is present before cancellation")
                        .write_all(&response)
                        .map_err(|error| {
                            CommandError::new(
                                "process_stdin_write_failed",
                                "Packwiz cancellation could not be sent",
                            )
                            .with_details(error.to_string())
                        })?;
                    stdin
                        .as_mut()
                        .expect("stdin is present before cancellation")
                        .flush()
                        .map_err(|error| {
                            CommandError::new(
                                "process_stdin_write_failed",
                                "Packwiz cancellation could not be flushed",
                            )
                            .with_details(error.to_string())
                        })?;
                    sent_n = true;
                    stdin.take();
                }
            }
            Ok(OutputChunk::Stderr(bytes)) => {
                append_bounded(
                    &mut stderr_text,
                    &bytes,
                    limits.max_output_bytes,
                    &mut output_truncated,
                );
                if !sent_n && machine.observe(&combined_output(&stdout_text, &stderr_text)) {
                    eprintln!("[discovery] expected prompt observed on stderr; sending n and closing stdin");
                    let response = machine.authorize_n()?;
                    stdin
                        .as_mut()
                        .expect("stdin is present before cancellation")
                        .write_all(&response)
                        .map_err(|error| {
                            CommandError::new(
                                "process_stdin_write_failed",
                                "Packwiz cancellation could not be sent",
                            )
                            .with_details(error.to_string())
                        })?;
                    stdin
                        .as_mut()
                        .expect("stdin is present before cancellation")
                        .flush()
                        .map_err(|error| {
                            CommandError::new(
                                "process_stdin_write_failed",
                                "Packwiz cancellation could not be flushed",
                            )
                            .with_details(error.to_string())
                        })?;
                    sent_n = true;
                    stdin.take();
                }
            }
            Ok(OutputChunk::Closed) => {
                closed_readers += 1;
                eprintln!("[discovery] output reader closed count={closed_readers}");
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => {
                eprintln!("[discovery] output channel disconnected; checking child status");
                let mut status = child.try_wait().ok().flatten();
                if status.is_none() {
                    for _ in 0..25 {
                        thread::sleep(Duration::from_millis(10));
                        status = child.try_wait().ok().flatten();
                        if status.is_some() {
                            eprintln!("[discovery] child exit observed after channel disconnect");
                            break;
                        }
                    }
                }
                let combined_output = combined_output(&stdout_text, &stderr_text);
                let cancellation = compatibility::classify_cancellation(
                    &combined_output,
                    status.as_ref().and_then(|value| value.code()),
                    &machine.state,
                );
                return Ok(ProcessEvidence {
                    executable: plan.executable.to_string_lossy().into_owned(),
                    arguments: plan.arguments.clone(),
                    working_directory: plan.working_directory.to_string_lossy().into_owned(),
                    stdout: stdout_text,
                    stderr: stderr_text,
                    exit_code: status.and_then(|value| value.code()),
                    started_at,
                    finished_at: Some(timestamp()),
                    output_truncated,
                    prompt: machine.state,
                    cancellation,
                });
            }
        }
    }
}
fn combined_output(stdout: &str, stderr: &str) -> String {
    if stdout.is_empty() {
        return stderr.to_string();
    }
    if stderr.is_empty() {
        return stdout.to_string();
    }
    format!("{stdout}\n{stderr}")
}

enum OutputKind {
    Stdout,
    Stderr,
}
fn spawn_reader<R: Read + Send + 'static>(
    mut reader: R,
    kind: OutputKind,
    sender: mpsc::Sender<OutputChunk>,
) {
    thread::spawn(move || {
        let mut buffer = [0u8; 4096];
        loop {
            match reader.read(&mut buffer) {
                Ok(0) => break,
                Ok(size) => {
                    let chunk = match kind {
                        OutputKind::Stdout => OutputChunk::Stdout(buffer[..size].to_vec()),
                        OutputKind::Stderr => OutputChunk::Stderr(buffer[..size].to_vec()),
                    };
                    if sender.send(chunk).is_err() {
                        return;
                    }
                }
                Err(_) => break,
            }
        }
        let _ = sender.send(OutputChunk::Closed);
    });
}
fn append_bounded(target: &mut String, bytes: &[u8], max: usize, truncated: &mut bool) {
    let remaining = max.saturating_sub(target.len());
    let take = remaining.min(bytes.len());
    target.push_str(&String::from_utf8_lossy(&bytes[..take]));
    if take < bytes.len() {
        *truncated = true;
    }
}
fn terminate_child(child: &mut std::process::Child) {
    #[cfg(windows)]
    {
        let _ = Command::new("taskkill")
            .args(["/PID", &child.id().to_string(), "/T", "/F"])
            .status();
    }
    let _ = child.kill();
    let _ = child.wait();
}
fn timestamp() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_secs().to_string())
        .unwrap_or_else(|_| "0".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn plan_is_separated_and_safe() {
        let plan = command_plan(r"C:\Packwiz\packwiz.exe", r"C:\Projects\pack").unwrap();
        assert_eq!(plan.arguments, ["update", "-a"]);
        assert!(!plan
            .arguments
            .iter()
            .any(|arg| arg == "y" || arg == "--yes"));
    }
    #[test]
    fn prompt_machine_only_emits_n_after_expected_prompt() {
        let mut machine = PromptMachine::default();
        assert!(machine.authorize_n().is_err());
        assert!(machine.observe("Updates found: Do you want to update? [Y/n]:"));
        assert_eq!(machine.authorize_n().unwrap(), b"n\n");
    }
    #[test]
    fn coordinator_blocks_same_project_and_releases_on_drop() {
        let coordinator = OperationCoordinator::default();
        let lease = coordinator.acquire("project").unwrap();
        assert!(coordinator.acquire("project").is_err());
        drop(lease);
        assert!(coordinator.acquire("project").is_ok());
    }
}
