use std::process::{Command, Stdio};
use std::io;

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
            use nix::sys::signal::{kill, Signal};
            use nix::unistd::Pid;

            kill(Pid::from_raw(self.pid as i32), Signal::SIGKILL)
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e))
        }

        #[cfg(windows)]
        {
            use windows_sys::Win32::System::Threading::*;
            use windows_sys::Win32::Foundation::CloseHandle;

            unsafe {
                let handle = OpenProcess(PROCESS_TERMINATE, 0, self.pid);
                if handle == 0 {
                    return Err(io::Error::last_os_error());
                }
                let result = TerminateProcess(handle, 1);
                CloseHandle(handle);
                if result == 0 {
                    return Err(io::Error::last_os_error());
                }
                Ok(())
            }
        }
    }
}
