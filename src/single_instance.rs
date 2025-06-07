pub use platform::*;

#[cfg(target_os = "windows")]
pub mod platform {
    use std::env::current_exe;

    use windows::Win32::Foundation::*;
    use windows::Win32::System::Memory::*;
    use windows::Win32::System::Threading::*;
    use windows::core::PCSTR;

    pub struct SingleInstance {
        h_mapping: HANDLE,
    }

    impl SingleInstance {
        pub fn try_new(keep_new_instance: bool) -> Result<Self, Box<dyn std::error::Error>> {
            let this_pid = std::process::id();
            let pid_size = size_of_val(&this_pid);

            let mapping_name = format!("Global\\{}\0", current_exe()?.file_name().unwrap().display());

            unsafe {
                let h_mapping = CreateFileMappingA(INVALID_HANDLE_VALUE, None, PAGE_READWRITE, 0, pid_size as _, PCSTR(mapping_name.as_ptr()))?;
                let mapped_buffer = MapViewOfFile(h_mapping, FILE_MAP_READ | FILE_MAP_WRITE, 0, 0, pid_size);
                let mapped_value = mapped_buffer.Value as *mut _;

                if GetLastError() == ERROR_ALREADY_EXISTS {
                    let other_pid = *mapped_value;
                    assert_ne!(other_pid, 0);
                    assert_ne!(other_pid, this_pid);

                    if keep_new_instance {
                        let h_other_proc = OpenProcess(PROCESS_TERMINATE, false, other_pid)?;
                        TerminateProcess(h_other_proc, 0)?;
                    } else {
                        std::process::exit(0);
                    }
                }

                *mapped_value = this_pid;
                UnmapViewOfFile(mapped_buffer)?;

                Ok(SingleInstance { h_mapping })
            }
        }
    }

    impl Drop for SingleInstance {
        fn drop(&mut self) {
            unsafe {
                CloseHandle(self.h_mapping).ok();
            }
        }
    }
}

#[cfg(target_os = "linux")]
mod platform {
    use std::env::{current_exe, temp_dir};
    use std::fs::{File, OpenOptions};
    use std::io::{Read, Seek, Write};

    use nix::errno::Errno;
    use nix::fcntl::{Flock, FlockArg};
    use nix::sys::signal::{Signal, kill};
    use nix::unistd::Pid;

    pub struct SingleInstance {
        _file_lock: Flock<File>,
    }

    impl SingleInstance {
        pub fn try_new(keep_new_instance: bool) -> Result<Self, Box<dyn std::error::Error>> {
            let this_pid = Pid::this();
            let pid_size = size_of_val(&this_pid);

            let lock_file_name = format!("{}/{}_single_instance.lock", temp_dir().display(), current_exe()?.file_name().unwrap().display());
            let lock_file = OpenOptions::new().read(true).write(true).create(true).open(&lock_file_name)?;

            let (mut file_lock, is_first) = match Flock::lock(lock_file, FlockArg::LockExclusiveNonblock) {
                Ok(lock) => {
                    lock.relock(FlockArg::LockSharedNonblock)?;
                    lock.set_len(pid_size as _)?;

                    (lock, true)
                }
                Err((f, Errno::EAGAIN)) => (Flock::lock(f, FlockArg::LockSharedNonblock).map_err(|(_, e)| e)?, false),
                Err((_, e)) => {
                    panic!("flock failed with errno: {e}");
                }
            };

            let mut pid_buffer = vec![0; pid_size];
            file_lock.read_exact(&mut pid_buffer)?;
            let other_pid = Pid::from_raw(libc::pid_t::from_le_bytes(pid_buffer.try_into().unwrap()));

            if !is_first {
                assert_ne!(other_pid.as_raw(), 0);
                assert_ne!(other_pid, this_pid);

                if keep_new_instance {
                    kill(other_pid, Signal::SIGTERM).ok();
                } else {
                    std::process::exit(0);
                }
            }

            file_lock.rewind()?;
            file_lock.write(&this_pid.as_raw().to_le_bytes())?;

            Ok(Self { _file_lock: file_lock })
        }
    }
}
