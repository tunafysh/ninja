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
            const CREATE_NEW_PROCESS_GROUP: u32 = 0x00000200;
            command.creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP);
        }

        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            unsafe {
                command.pre_exec(|| {
                    libc::setsid();
                    Ok(())
                });
            }
        }

        let mut child = command.spawn()?;
        let pid = child.id();
        
        #[cfg(unix)]
        {
            std::mem::forget(child);
        }
        
        #[cfg(windows)]
        {
            std::mem::forget(child);
        }
        
        Ok(Self { pid })
    }

    /// Attempt graceful termination before forcefully killing
    pub fn kill(&self) -> io::Result<()> {
        #[cfg(unix)]
        {
            use nix::sys::signal::{Signal, kill};
            use nix::unistd::Pid;
            use std::thread;
            use std::time::Duration;

            let pid = Pid::from_raw(self.pid as i32);
            
            if kill(pid, Signal::SIGTERM).is_ok() {
                for _ in 0..10 {
                    thread::sleep(Duration::from_millis(100));
                    if kill(pid, None).is_err() {
                        return Ok(());
                    }
                }
            }
            
            kill(pid, Signal::SIGKILL).map_err(io::Error::other)
        }

        #[cfg(windows)]
        {
            use windows::Win32::Foundation::CloseHandle;
            use windows::Win32::System::Threading::*;

            unsafe {
                let handle = OpenProcess(PROCESS_TERMINATE, false, self.pid)
                    .map_err(|e| io::Error::new(io::ErrorKind::PermissionDenied, e))?;
                
                let result = TerminateProcess(handle, 1)
                    .map_err(|e| io::Error::new(io::ErrorKind::Other, e));
                
                let _ = CloseHandle(handle);
                result
            }
        }
    }
}
