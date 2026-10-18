#![allow(dead_code)]

use keyring::Entry;

const SERVICE_PREFIX: &str = "com.edgeagent.app";

pub fn save_credential(id: &str, _service: &str, username: &str, password: &str) -> Result<(), String> {
    let entry = Entry::new(&format!("{}.{}", SERVICE_PREFIX, id), username)
        .map_err(|e| format!("创建钥匙串条目失败: {}", e))?;
    entry.set_password(password)
        .map_err(|e| format!("保存密码失败: {}", e))?;
    log::info!("凭据 [{}] 已保存", id);
    Ok(())
}

pub fn get_password(id: &str, username: &str) -> Result<String, String> {
    let entry = Entry::new(&format!("{}.{}", SERVICE_PREFIX, id), username)
        .map_err(|e| format!("读取钥匙串条目失败: {}", e))?;
    entry.get_password()
        .map_err(|e| format!("读取密码失败: {}", e))
}

pub fn delete_credential(id: &str, username: &str) -> Result<(), String> {
    let entry = Entry::new(&format!("{}.{}", SERVICE_PREFIX, id), username)
        .map_err(|e| format!("删除钥匙串条目失败: {}", e))?;
    entry.delete_credential()
        .map_err(|e| format!("删除密码失败: {}", e))
}
