//! Audited Win32 boundary for synthetic IPC only. No sandbox, service or token modification.
use agentguard_protocol::{ProtocolError, Result, crypto::hex};
use sha2::{Digest, Sha256};
use std::ffi::c_void;
use std::fs::File;
use std::io::Read;
use std::mem::{size_of, zeroed};
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
use std::ptr::null_mut;
use tokio::net::windows::named_pipe::{NamedPipeClient, NamedPipeServer, ServerOptions};
use windows_sys::Win32::Foundation::{FILETIME, HANDLE, LocalFree, WAIT_TIMEOUT};
use windows_sys::Win32::Security::Authorization::{
    ConvertSidToStringSidW, ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1,
};
use windows_sys::Win32::Security::Cryptography::{
    BCRYPT_USE_SYSTEM_PREFERRED_RNG, BCryptGenRandom,
};
use windows_sys::Win32::Security::*;
use windows_sys::Win32::System::Pipes::{GetNamedPipeClientProcessId, GetNamedPipeServerProcessId};
use windows_sys::Win32::System::Threading::*;

fn failure() -> ProtocolError {
    ProtocolError::new(
        "peer_identity",
        "无法核实当前连接的进程身份，本次请求已拒绝。",
    )
}

fn bool_ok(value: i32) -> Result<()> {
    if value == 0 { Err(failure()) } else { Ok(()) }
}

fn owned(handle: HANDLE) -> Result<OwnedHandle> {
    if handle.is_null() {
        return Err(failure());
    }
    // SAFETY: the preceding successful Win32 call transfers ownership of this real handle.
    Ok(unsafe { OwnedHandle::from_raw_handle(handle) })
}

fn token_data(token: &OwnedHandle, class: TOKEN_INFORMATION_CLASS) -> Result<Vec<u8>> {
    let mut needed = 0;
    // SAFETY: first call only queries required size. Null output has zero length.
    unsafe { GetTokenInformation(token.as_raw_handle(), class, null_mut(), 0, &mut needed) };
    if needed == 0 || needed > 65536 {
        return Err(failure());
    }
    let mut bytes = vec![0; needed as usize];
    // SAFETY: buffer has the exact capacity requested by GetTokenInformation.
    bool_ok(unsafe {
        GetTokenInformation(
            token.as_raw_handle(),
            class,
            bytes.as_mut_ptr().cast(),
            needed,
            &mut needed,
        )
    })?;
    Ok(bytes)
}

fn sid_string(sid: PSID) -> Result<String> {
    let mut text = null_mut();
    // SAFETY: SID comes from a live GetTokenInformation buffer during this call.
    bool_ok(unsafe { ConvertSidToStringSidW(sid, &mut text) })?;
    let mut length = 0;
    // SAFETY: the API returned a null-terminated allocation. Bound SID text to 184 units.
    unsafe {
        while length < 184 && *text.add(length) != 0 {
            length += 1;
        }
        let result = String::from_utf16(std::slice::from_raw_parts(text, length));
        LocalFree(text.cast());
        result.map_err(|_| failure())
    }
}

fn luid(value: windows_sys::Win32::Foundation::LUID) -> u64 {
    ((value.HighPart as u32 as u64) << 32) | u64::from(value.LowPart)
}

fn image_digest(process: &OwnedHandle) -> Result<String> {
    let mut buffer = vec![0u16; 32768];
    let mut size = buffer.len() as u32;
    // SAFETY: process is live and buffer is writable with length size.
    bool_ok(unsafe {
        QueryFullProcessImageNameW(process.as_raw_handle(), 0, buffer.as_mut_ptr(), &mut size)
    })?;
    let path = String::from_utf16(&buffer[..size as usize]).map_err(|_| failure())?;
    const MAX_IMAGE_BYTES: u64 = 256 * 1024 * 1024;
    let file = File::open(path).map_err(|_| failure())?;
    if file.metadata().map_err(|_| failure())?.len() > MAX_IMAGE_BYTES {
        return Err(failure());
    }
    let mut file = file.take(MAX_IMAGE_BYTES + 1);
    let mut digest = Sha256::new();
    let mut block = [0; 65536];
    let mut total = 0;
    loop {
        let count = file.read(&mut block).map_err(|_| failure())?;
        if count == 0 {
            break;
        }
        total += count as u64;
        if total > MAX_IMAGE_BYTES {
            return Err(failure());
        }
        digest.update(&block[..count]);
    }
    Ok(hex(&digest.finalize()))
}

pub struct Identity {
    process: OwnedHandle,
    pub pid: u32,
    created: u64,
    sid: String,
    logon_sid: String,
    authentication_luid: u64,
    modified_luid: u64,
    integrity_sid: String,
    restricted: bool,
    pub image_sha256: String,
}

impl Identity {
    pub fn capture(pid: u32) -> Result<Self> {
        // SAFETY: read-only query rights plus wait access. No write/terminate/token modification.
        let process = owned(unsafe {
            OpenProcess(
                PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SYNCHRONIZE,
                0,
                pid,
            )
        })?;
        let mut raw_token = null_mut();
        // SAFETY: output pointer is valid and process is owned.
        bool_ok(unsafe { OpenProcessToken(process.as_raw_handle(), TOKEN_QUERY, &mut raw_token) })?;
        let token = owned(raw_token)?;
        let user_buffer = token_data(&token, TokenUser)?;
        let groups_buffer = token_data(&token, TokenLogonSid)?;
        let statistics_buffer = token_data(&token, TokenStatistics)?;
        let integrity_buffer = token_data(&token, TokenIntegrityLevel)?;
        if user_buffer.len() < size_of::<TOKEN_USER>()
            || groups_buffer.len() < size_of::<TOKEN_GROUPS>()
            || statistics_buffer.len() < size_of::<TOKEN_STATISTICS>()
            || integrity_buffer.len() < size_of::<TOKEN_MANDATORY_LABEL>()
        {
            return Err(failure());
        }
        // SAFETY: byte lengths checked. read_unaligned avoids relying on Vec<u8> alignment.
        let (user, groups, statistics, integrity) = unsafe {
            (
                std::ptr::read_unaligned(user_buffer.as_ptr().cast::<TOKEN_USER>()),
                std::ptr::read_unaligned(groups_buffer.as_ptr().cast::<TOKEN_GROUPS>()),
                std::ptr::read_unaligned(statistics_buffer.as_ptr().cast::<TOKEN_STATISTICS>()),
                std::ptr::read_unaligned(integrity_buffer.as_ptr().cast::<TOKEN_MANDATORY_LABEL>()),
            )
        };
        if groups.GroupCount != 1 {
            return Err(failure());
        }
        let sid = sid_string(user.User.Sid)?;
        let logon_sid = sid_string(groups.Groups[0].Sid)?;
        let integrity_sid = sid_string(integrity.Label.Sid)?;
        // SAFETY: all output FILETIMEs are initialized and writable.
        let (mut created, mut exit, mut kernel, mut user_time): (
            FILETIME,
            FILETIME,
            FILETIME,
            FILETIME,
        ) = unsafe { zeroed() };
        bool_ok(unsafe {
            GetProcessTimes(
                process.as_raw_handle(),
                &mut created,
                &mut exit,
                &mut kernel,
                &mut user_time,
            )
        })?;
        Ok(Self {
            image_sha256: image_digest(&process)?,
            process,
            pid,
            created: (u64::from(created.dwHighDateTime) << 32) | u64::from(created.dwLowDateTime),
            sid,
            logon_sid,
            authentication_luid: luid(statistics.AuthenticationId),
            modified_luid: luid(statistics.ModifiedId),
            integrity_sid,
            // SAFETY: token remains live during the query.
            restricted: unsafe { IsTokenRestricted(token.as_raw_handle()) } != 0,
        })
    }

    pub fn same_principal(&self, other: &Self) -> bool {
        self.sid == other.sid
            && self.logon_sid == other.logon_sid
            && self.authentication_luid == other.authentication_luid
            && self.integrity_sid == other.integrity_sid
            && self.restricted == other.restricted
    }

    pub fn revalidate(&self) -> Result<()> {
        // SAFETY: the owned process handle has SYNCHRONIZE access and is retained for this connection.
        if unsafe { WaitForSingleObject(self.process.as_raw_handle(), 0) } != WAIT_TIMEOUT {
            return Err(failure());
        }
        let current = Self::capture(self.pid)?;
        if self.created != current.created
            || self.modified_luid != current.modified_luid
            || self.image_sha256 != current.image_sha256
            || !self.same_principal(&current)
        {
            return Err(failure());
        }
        Ok(())
    }
}

fn peer_pid(pipe: HANDLE, client: bool) -> Result<u32> {
    let mut pid = 0;
    // SAFETY: caller holds the live NamedPipe handle. PID is obtained from the OS, not payload.
    let success = unsafe {
        if client {
            GetNamedPipeClientProcessId(pipe, &mut pid)
        } else {
            GetNamedPipeServerProcessId(pipe, &mut pid)
        }
    };
    bool_ok(success)?;
    Ok(pid)
}

pub fn client_pid(pipe: &NamedPipeServer) -> Result<u32> {
    peer_pid(pipe.as_raw_handle(), true)
}

pub fn server_pid(pipe: &NamedPipeClient) -> Result<u32> {
    peer_pid(pipe.as_raw_handle(), false)
}

pub fn challenge() -> Result<String> {
    let mut bytes = [0; 32];
    // SAFETY: system-preferred RNG requires null algorithm and a 32-byte writable buffer.
    let status = unsafe {
        BCryptGenRandom(
            null_mut(),
            bytes.as_mut_ptr(),
            32,
            BCRYPT_USE_SYSTEM_PREFERRED_RNG,
        )
    };
    if status < 0 {
        return Err(ProtocolError::new(
            "rng_unavailable",
            "无法建立安全连接，本次请求已拒绝。",
        ));
    }
    Ok(hex(&bytes))
}

pub fn validate_pipe_name(name: &str) -> Result<()> {
    let suffix = name
        .strip_prefix(r"\\.\pipe\AgentGuard.Synthetic.")
        .ok_or_else(failure)?;
    uuid::Uuid::parse_str(suffix).map_err(|_| failure())?;
    Ok(())
}

struct LocalAllocation(*mut c_void);
impl Drop for LocalAllocation {
    fn drop(&mut self) {
        // SAFETY: conversion API allocated this descriptor with LocalAlloc.
        unsafe {
            LocalFree(self.0);
        }
    }
}

pub fn create_pipe(name: &str, identity: &Identity) -> Result<NamedPipeServer> {
    validate_pipe_name(name)?;
    let sddl: Vec<u16> = format!("D:P(A;;GA;;;{})", identity.logon_sid)
        .encode_utf16()
        .chain([0])
        .collect();
    let mut descriptor = null_mut();
    // SAFETY: SDDL is null-terminated, output is owned by LocalAllocation below.
    bool_ok(unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            sddl.as_ptr(),
            SDDL_REVISION_1,
            &mut descriptor,
            null_mut(),
        )
    })?;
    let descriptor = LocalAllocation(descriptor);
    let mut attributes = SECURITY_ATTRIBUTES {
        nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: descriptor.0,
        bInheritHandle: 0,
    };
    // SAFETY: descriptor and SECURITY_ATTRIBUTES stay alive until CreateNamedPipe returns.
    unsafe {
        ServerOptions::new()
            .first_pipe_instance(true)
            .reject_remote_clients(true)
            .max_instances(1)
            .create_with_security_attributes_raw(
                name,
                (&mut attributes as *mut SECURITY_ATTRIBUTES).cast(),
            )
    }
    .map_err(|_| ProtocolError::new("pipe_create", "无法创建专用连接，名称可能已被占用。"))
}
