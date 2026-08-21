use std::collections::HashMap;
use std::sync::OnceLock;

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum Role {
    Admin,
    Developer,
    Guest,
}

pub struct RbacSystem {
    role_permissions: HashMap<Role, Vec<&'static str>>,
}

impl RbacSystem {
    pub fn global() -> &'static RbacSystem {
        static RBAC: OnceLock<RbacSystem> = OnceLock::new();
        RBAC.get_or_init(|| {
            let mut permissions = HashMap::new();
            
            // Admin имеет доступ ко всем инструментам без исключения
            permissions.insert(Role::Admin, vec!["list_dir", "read_file", "write_file", "sys_info", "wasm_exec"]);
            
            // Developer может читать, смотреть систему и запускать WASM, но не писать критические файлы
            permissions.insert(Role::Developer, vec!["list_dir", "read_file", "sys_info", "wasm_exec"]);
            
            // Guest имеет минимальные права только на чтение информации
            permissions.insert(Role::Guest, vec!["sys_info"]);

            RbacSystem { role_permissions: permissions }
        })
    }

    /// Проверка, разрешен ли конкретной роли вызов определенного инструмента
    pub fn check_permission(&self, role: Role, tool_name: &str) -> bool {
        if let Some(allowed_tools) = self.role_permissions.get(&role) {
            return allowed_tools.contains(&tool_name);
        }
        false
    }
}