//! Windows process enumeration + command-line reading via direct Win32 shims.
//!
//! Used to find Claude Desktop instances bound to a guise profile: the
//! `--user-data-dir=<dir>` token in the process command line. No extra
//! crates — just `kernel32`/`ntdll` entry points.
//!
//! x86_64 only: reading another process's command line relies on documented
//! x64 PEB/`RTL_USER_PROCESS_PARAMETERS` field offsets.

#[cfg(not(target_arch = "x86_64"))]
compile_error!("guise's Windows process reader needs x86_64 PEB offsets");

use anyhow::{Context, Result};

type Dword = u32;
type Handle = *mut std::ffi::c_void;
type Bool = i32;

const TH32CS_SNAPPROCESS: Dword = 0x0000_0002;
const PROCESS_QUERY_LIMITED_INFORMATION: Dword = 0x1000;
const PROCESS_VM_READ: Dword = 0x0010;
/// `PROCESSINFOCLASS::ProcessBasicInformation`.
const PROCESS_BASIC_INFORMATION: i32 = 0;
/// `PEB.ProcessParameters` offset on x86_64.
const PEB_PROCESS_PARAMETERS: usize = 0x20;
/// `RTL_USER_PROCESS_PARAMETERS.CommandLine` offset on x86_64.
const PARAMS_COMMAND_LINE: usize = 0x70;
/// Longest command line we will read from another process.
const MAX_CMDLINE_BYTES: usize = 32 * 1024;

#[repr(C)]
struct ProcessEntry32W {
    dw_size: Dword,
    cnt_usage: Dword,
    th32_process_id: Dword,
    th32_default_heap_id: usize,
    th32_module_id: Dword,
    cnt_threads: Dword,
    th32_parent_process_id: Dword,
    pc_pri_class_base: i32,
    dw_flags: Dword,
    sz_exe_file: [u16; 260],
}

#[repr(C)]
struct ProcessBasicInformation {
    exit_status: i32,
    peb_base: *const u8,
    affinity_mask: usize,
    base_priority: i32,
    unique_pid: usize,
    inherited_from_pid: usize,
}

#[link(name = "kernel32")]
extern "system" {
    fn CreateToolhelp32Snapshot(flags: Dword, pid: Dword) -> Handle;
    fn Process32FirstW(snapshot: Handle, entry: *mut ProcessEntry32W) -> Bool;
    fn Process32NextW(snapshot: Handle, entry: *mut ProcessEntry32W) -> Bool;
    fn CloseHandle(handle: Handle) -> Bool;
    fn OpenProcess(access: Dword, inherit: Bool, pid: Dword) -> Handle;
    fn ReadProcessMemory(
        process: Handle,
        base: *const u8,
        buf: *mut u8,
        size: usize,
        read: *mut usize,
    ) -> Bool;
    fn GetLastError() -> Dword;
}

#[link(name = "ntdll")]
extern "system" {
    fn NtQueryInformationProcess(
        process: Handle,
        class: i32,
        info: *mut u8,
        len: u32,
        ret_len: *mut u32,
    ) -> i32;
}

/// RAII closer so early returns can't leak handles.
struct OwnedHandle(Handle);

impl Drop for OwnedHandle {
    fn drop(&mut self) {
        if !self.0.is_null() && self.0 as isize != -1 {
            unsafe {
                CloseHandle(self.0);
            }
        }
    }
}

fn decode_utf16_nul(buf: &[u16]) -> String {
    let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..end])
}

/// Every visible process as `(pid, exe file name)`.
pub fn all_processes() -> Result<Vec<(u32, String)>> {
    unsafe {
        let snap = OwnedHandle(CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0));
        if snap.0 as isize == -1 {
            anyhow::bail!("process snapshot failed: {}", GetLastError());
        }
        let mut entry: ProcessEntry32W = std::mem::zeroed();
        entry.dw_size = std::mem::size_of::<ProcessEntry32W>() as Dword;
        let mut out = Vec::new();
        if Process32FirstW(snap.0, &mut entry) != 0 {
            loop {
                out.push((entry.th32_process_id, decode_utf16_nul(&entry.sz_exe_file)));
                if Process32NextW(snap.0, &mut entry) == 0 {
                    break;
                }
            }
        }
        Ok(out)
    }
}

fn read_remote(handle: Handle, base: *const u8, buf: &mut [u8]) -> Result<()> {
    unsafe {
        let mut read = 0usize;
        let ok = ReadProcessMemory(handle, base, buf.as_mut_ptr(), buf.len(), &mut read);
        if ok == 0 || read != buf.len() {
            anyhow::bail!("remote read failed: {}", GetLastError());
        }
        Ok(())
    }
}

/// Full command line of `pid`, via PEB → ProcessParameters → CommandLine.
pub fn command_line(pid: u32) -> Result<String> {
    unsafe {
        let proc = OwnedHandle(OpenProcess(
            PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_VM_READ,
            0,
            pid,
        ));
        if proc.0.is_null() {
            anyhow::bail!("open pid {pid}: {}", GetLastError());
        }
        let mut pbi: ProcessBasicInformation = std::mem::zeroed();
        let mut ret_len = 0u32;
        let status = NtQueryInformationProcess(
            proc.0,
            PROCESS_BASIC_INFORMATION,
            &mut pbi as *mut ProcessBasicInformation as *mut u8,
            std::mem::size_of::<ProcessBasicInformation>() as u32,
            &mut ret_len,
        );
        if status != 0 {
            anyhow::bail!("query pid {pid}: ntstatus {status:#x}");
        }
        if pbi.peb_base.is_null() {
            anyhow::bail!("query pid {pid}: no PEB");
        }
        let mut addr = [0u8; 8];
        read_remote(proc.0, pbi.peb_base.add(PEB_PROCESS_PARAMETERS), &mut addr)
            .with_context(|| format!("read params addr pid {pid}"))?;
        let params = u64::from_ne_bytes(addr) as *const u8;
        if params.is_null() {
            anyhow::bail!("read params pid {pid}: null");
        }
        let mut us = [0u8; 16];
        read_remote(proc.0, params.add(PARAMS_COMMAND_LINE), &mut us)
            .with_context(|| format!("read cmdline header pid {pid}"))?;
        let len = u16::from_ne_bytes([us[0], us[1]]) as usize;
        let buf = u64::from_ne_bytes(us[8..16].try_into().unwrap()) as *const u8;
        if len == 0 || buf.is_null() {
            return Ok(String::new());
        }
        let mut raw = vec![0u8; len.min(MAX_CMDLINE_BYTES)];
        read_remote(proc.0, buf, &mut raw)
            .with_context(|| format!("read cmdline body pid {pid}"))?;
        let wide: Vec<u16> = raw
            .chunks_exact(2)
            .map(|c| u16::from_ne_bytes([c[0], c[1]]))
            .collect();
        Ok(String::from_utf16_lossy(&wide))
    }
}

/// PIDs of processes whose command line binds them to exactly this userData
/// dir. Matches on the full `--user-data-dir=<dir>` token rather than the
/// exe name: the direct and Store builds ship different executables, but the
/// token is unambiguous. Unreadable processes are skipped, never fatal.
pub fn pids_for_data_dir(data_dir: &std::path::Path) -> Result<Vec<u32>> {
    let needle = format!("--user-data-dir={}", data_dir.display());
    let me = std::process::id();
    let mut out = Vec::new();
    for (pid, _exe) in all_processes()? {
        if pid == me {
            continue;
        }
        match command_line(pid) {
            Ok(cmd) if cmd.contains(&needle) => out.push(pid),
            _ => {}
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn census_contains_self() {
        let me = std::process::id();
        let all = all_processes().unwrap();
        assert!(!all.is_empty());
        assert!(all.iter().any(|(pid, _)| *pid == me));
    }

    #[test]
    fn own_cmdline_is_readable() {
        let cmd = command_line(std::process::id()).unwrap();
        let exe = std::env::current_exe().unwrap();
        let name = exe.file_name().and_then(|n| n.to_str()).unwrap();
        assert!(
            cmd.contains(name),
            "own cmdline should name the test binary: {cmd:?}"
        );
    }

    #[test]
    fn unknown_data_dir_matches_nothing() {
        let pids = pids_for_data_dir(std::path::Path::new(
            r"C:\guise-definitely-not-a-real-profile-9f3c",
        ))
        .unwrap();
        assert!(pids.is_empty());
    }
}
