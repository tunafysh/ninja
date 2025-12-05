use std::io;
use std::process::{Command, Stdio};

#[derive(Debug)]
pub struct DetachedProcess {
    pub pid: u32,
}

impl DetachedProcess {
    /// Spawn a truly detached process
    pub fn spawn(cmd: &str, args: &[&str]) -> io::Result<Self> {
        let mut command = Command::new(cmd);
        command
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());

        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const DETACHED_PROCESS: u32 = 0x00000008;
            command.creation_flags(DETACHED_PROCESS);
        }

        #[cfg(unix)]
        {
            use libc;
            use std::os::unix::process::CommandExt;
            unsafe {
                command.pre_exec(|| {
                    libc::setsid(); // detach fully
                    Ok(())
                });
            }
        }

        let child = command.spawn()?; // fire & forget
        Ok(Self { pid: child.id() })
    }

    /// Kill the detached process by PID
    pub fn kill(&self) -> io::Result<()> {
        #[cfg(unix)]
        {
            use nix::sys::signal::{Signal, kill};
            use nix::unistd::Pid;

            kill(Pid::from_raw(self.pid as i32), Signal::SIGKILL).map_err(io::Error::other)
        }

        #[cfg(windows)]
        {
            use windows::Win32::System::Threading::*;

            unsafe {
                let handle = OpenProcess(PROCESS_TERMINATE, false, self.pid);
                TerminateProcess(handle?, 1)?;
                Ok(())
            }
        }
    }
}
