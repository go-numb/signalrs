use std::{env, path::Path};

use chrono::{DateTime, Utc};
use log::error;

use crate::middleware::file;

pub fn sleep(sec: u64, ms: u64) {
    let wait_ms = sec * 1000 + ms;
    std::thread::sleep(std::time::Duration::from_millis(wait_ms));
}

pub fn ok(is_production: bool, sid: &str, jtc_s: &str) -> bool {
    let s = match win::get_computer_identifier() {
        Ok(s) => s,
        Err(e) => {
            error!("does not get compute identifier: {}", e);
            return false;
        }
    };

    // 含まれるかどうか
    // 本番環境でない場合は確認不要
    if is_production && !sid.contains(s.as_str()) {
        error!("{} is not included in {}", sid, s);
        return false;
    }

    if expired(jtc_s) {
        error!("expired this program, contact to developer");
        return false;
    }

    true
}

/// 有効期限切れかどうかを判定する
fn expired(jtc_s: &str) -> bool {
    let mut expire_at = DateTime::parse_from_rfc3339(jtc_s).unwrap();
    expire_at += chrono::Duration::hours(-9);
    let now = Utc::now();

    now > expire_at.to_utc()
}

/// ロガーの設定を行う
pub fn set_env_for_logger(level: &str) {
    env::set_var("RUST_LOG", level);
    env_logger::init();
}

/// 設定ファイルの保存先を設定する
pub fn set_target_save_file(s: &str) -> String {
    let current_dir = env::current_dir().unwrap();
    let target_save_file = current_dir.join(s).display().to_string();

    // ディレクトリが存在しない場合は作成する
    let dir = Path::new(target_save_file.as_str()).parent().unwrap();
    file::create_save_dir(dir).unwrap();
    target_save_file
}

pub mod win {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;
    use std::ptr;
    use winapi::shared::sddl::ConvertSidToStringSidW;
    use winapi::um::processthreadsapi::*;
    use winapi::um::securitybaseapi::*;
    use winapi::um::winbase::LocalFree;
    use winapi::um::winnt::*;

    pub fn get_computer_identifier() -> Result<String, String> {
        unsafe {
            let mut token: HANDLE = ptr::null_mut();
            if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 {
                return Err("Failed to open process token".to_string());
            }

            let mut size = 0;
            GetTokenInformation(token, TokenUser, ptr::null_mut(), 0, &mut size);

            let mut buffer = vec![0u8; size as usize];
            if GetTokenInformation(
                token,
                TokenUser,
                buffer.as_mut_ptr() as *mut _,
                size,
                &mut size,
            ) == 0
            {
                return Err("Failed to get token information".to_string());
            }

            let token_user = &*(buffer.as_ptr() as *const TOKEN_USER);

            let mut sid_ptr: LPWSTR = ptr::null_mut();
            if ConvertSidToStringSidW(token_user.User.Sid, &mut sid_ptr) == 0 {
                return Err("Failed to convert SID to string".to_string());
            }

            let sid = {
                let len = (0..).take_while(|&i| *sid_ptr.offset(i) != 0).count();
                let slice = std::slice::from_raw_parts(sid_ptr, len);
                OsString::from_wide(slice).into_string().unwrap()
            };

            // SIDから必要な部分を抽出
            let parts: Vec<&str> = sid.split('-').collect();
            let result = if parts.len() >= 8 {
                Ok(format!("{}-{}-{}", parts[5], parts[6], parts[7]))
            } else {
                Err("Invalid SID format".to_string())
            };

            // メモリを解放
            LocalFree(sid_ptr as *mut _);

            result
        }
    }
}
